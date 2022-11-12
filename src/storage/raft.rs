use std::error::Error;
use std::fmt::{Debug, Display};
use std::io::Cursor;
use std::ops::RangeBounds;
use std::path::Path;
use std::sync::Arc;

use openraft::{AnyError, BasicNode, EffectiveMembership, Entry, EntryPayload, ErrorSubject, ErrorVerb, LeaderId, LogId, RaftLogReader, RaftSnapshotBuilder, RaftStorage, SnapshotMeta, StorageError, StorageIOError, Vote};
use openraft::storage::{LogState, Snapshot};
use parking_lot::RwLock;
use serde::{Serialize, Deserialize};

use crate::{NodeId, StorageHandle};
use crate::storage::params::TransportableParam;
use crate::storage::SqliteMemory;

openraft::declare_raft_types!(
    pub TypeConfig: D = Request, R = Response, NodeId = NodeId, Node = BasicNode
);

type StorageResult<T> = Result<T, StorageError<NodeId>>;

static SNAPSHOT_META_KEY: &str = "snapshot_meta";
static LAST_MEMBERSHIP_KEY: &str = "last_membership";
static LAST_LOG_KEY: &str = "last_applied_log";
static VOTE_KEY: &str = "vote";
static SNAPSHOT_INDEX_KEY: &str = "snapshot_index";
static LAST_PURGED_ID_KEY: &str = "last_purged_id";
static REPLICAT_KV_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS replicat_kv_states (
        key TEXT PRIMARY KEY,
        value
    );
"#;
static REPLICAT_RAFT_LOGS_TABLE: &str = r#"
    CREATE TABLE IF NOT EXISTS replicat_raft_logs (
        log_index BIGINT PRIMARY KEY,
        node_id BIGINT,
        term BIGINT,
        entry BLOB
    );
"#;
static REPLICAT_KV_SELECT_VALUE: &str = "SELECT value FROM replicat_kv_states WHERE key = ?;";
static REPLICAT_KV_UPSERT: &str = r#"
    INSERT INTO replicat_kv_states (key, value)
    VALUES (?, ?)
    ON CONFLICT(key) DO UPDATE SET
        value = excluded.value
    WHERE key = excluded.key;
"#;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Response {

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Request {
    Execute { sql: String, value: Vec<TransportableParam> },
}

/// Creates a on-disk snapshot of the current statemachine.
async fn create_snapshot(
    conn: &StorageHandle,
    from: &StorageHandle,
    meta: &SnapshotMeta<NodeId, BasicNode>,
) -> Result<SqliteMemory, anyhow::Error> {

    if let Some(data) = from.serialize().await? {
        conn.load_from_serialized(data).await?;
    }

    write_snapshot_meta(conn, meta).await?;

    Ok(conn.serialize().await?.unwrap())
}

async fn write_snapshot_meta(
    conn: &StorageHandle,
    meta: &SnapshotMeta<NodeId, BasicNode>,
) -> Result<(), anyhow::Error> {
    let meta = rmp_serde::to_vec(meta)?;
    conn.execute(REPLICAT_KV_UPSERT, (SNAPSHOT_META_KEY, meta)).await?;

    Ok(())
}

async fn get_snapshot_meta(conn: &StorageHandle) -> Result<Option<SnapshotMeta<NodeId, BasicNode>>, anyhow::Error> {
    let meta = conn
        .fetch_one::<_, (Vec<u8>,)>(REPLICAT_KV_SELECT_VALUE, (SNAPSHOT_META_KEY,))
        .await?
        .map(|data| rmp_serde::from_slice(&data.0))
        .transpose()?;

    Ok(meta)
}

#[derive(Debug)]
pub struct StateMachine {
    last_applied_log: RwLock<Option<LogId<NodeId>>>,

    last_membership: RwLock<EffectiveMembership<NodeId, BasicNode>>,

    /// The live application data.
    pub data: StorageHandle,
}

impl StateMachine {
    #[inline]
    pub fn last_applied_log(&self) -> Option<LogId<NodeId>> {
        *self.last_applied_log.read()
    }

    #[inline]
    pub fn last_membership(&self) -> EffectiveMembership<NodeId, BasicNode> {
        self.last_membership.read().clone()
    }

    pub async fn set_last_applied_log(&self, log: LogId<NodeId>) -> StorageResult<()> {
        let raw = rmp_serde::to_vec(&log)
            .map_err(store_write_error)?;

        self.data
            .execute(REPLICAT_KV_UPSERT, (LAST_LOG_KEY, raw))
            .await
            .map_err(store_write_error)?;

        let mut lock = self.last_applied_log.write();
        (*lock) = Some(log);

        Ok(())
    }

    pub async fn set_last_membership(&self, membership: EffectiveMembership<NodeId, BasicNode>) -> StorageResult<()> {
        let raw = rmp_serde::to_vec(&membership)
            .map_err(store_write_error)?;

        self.data
            .execute(REPLICAT_KV_UPSERT, (LAST_MEMBERSHIP_KEY, raw))
            .await
            .map_err(store_write_error)?;

        let mut lock = self.last_membership.write();
        (*lock) = membership;

        Ok(())
    }
}


#[derive(Debug)]
pub struct RaftStore {
    /// The persistent Raft log.
    log: StorageHandle,

    /// The current Raft state machine.
    state_machine: StateMachine,

    snapshot_handle: StorageHandle,
}

impl RaftStore {
    pub async fn open(
        base_path: &Path,
        state_store: &Path,
    ) -> Result<Self, anyhow::Error> {
        let log_store = base_path.join("log.db");
        let snapshot_store = base_path.join("snapshot.db");

        let log = StorageHandle::open(log_store).await?;
        let snapshot_handle = StorageHandle::open(snapshot_store).await?;
        let data = StorageHandle::open(state_store).await?;

        setup_log_store(&log).await?;
        setup_snapshot_store(&snapshot_handle).await?;
        let state_machine = create_state_machine(data).await?;

        Ok(Self {
            log,
            snapshot_handle,
            state_machine,
        })
    }

    async fn get_last_purged_log_id(&self) -> StorageResult<Option<LogId<NodeId>>> {
        self.log
            .fetch_one::<_, (Vec<u8>,)>(REPLICAT_KV_SELECT_VALUE, (LAST_PURGED_ID_KEY,))
            .await
            .map_err(store_read_error)?
            .map(|data| {
                rmp_serde::from_slice(&data.0)
                    .map_err(store_read_error)
            })
            .transpose()
    }

    async fn get_and_inc_snapshot_index(&self) -> StorageResult<u64> {
        let qry = "UPDATE replicat_kv_states SET value = value + 1 WHERE key = ? RETURNING value;";

        let snapshot_index = self.log
            .fetch_one::<_, (i64,)>(qry, (SNAPSHOT_INDEX_KEY,))
            .await
            .map_err(store_read_error)?
            .map(|v| v.0);

        if snapshot_index.is_none() {
            let qry = "INSERT INTO replicat_kv_states (key, value) VALUES (?, 0)";
            self.log
                .execute(qry, (SNAPSHOT_INDEX_KEY,))
                .await
                .map_err(store_read_error)?;
        }

        Ok(snapshot_index.unwrap_or(0) as u64)
    }

    async fn set_vote(&self, vote: &Vote<NodeId>) -> StorageResult<()> {
        let data = rmp_serde::to_vec(vote)
            .map_err(store_write_error)?;

        self.log
            .execute(REPLICAT_KV_UPSERT, (VOTE_KEY, data))
            .await
            .map_err(store_write_error)?;

        Ok(())
    }

    async fn get_vote(&self) -> StorageResult<Option<Vote<NodeId>>> {
        self.log
            .fetch_one::<_, (Vec<u8>,)>(REPLICAT_KV_SELECT_VALUE, (VOTE_KEY,))
            .await
            .map_err(store_read_error)?
            .map(|data| {
                rmp_serde::from_slice(&data.0)
                    .map_err(store_read_error)
            })
            .transpose()
    }

    async fn get_last_log_entry(&self) -> StorageResult<Option<LogId<NodeId>>> {
        let qry = "SELECT node_id, log_index, term FROM replicat_raft_logs ORDER BY log_index DESC LIMIT 1;";

        let log_id = self.log
            .fetch_one::<_, (i64, i64, i64)>(qry, ())
            .await
            .map_err(log_read_error)?
            .map(|(node_id, index, term)| {
                LogId {
                    leader_id: LeaderId {
                        node_id: node_id as u64,
                        term: term as u64,
                    },
                    index: index as u64
                }
            });

        Ok(log_id)
    }

}

#[async_trait::async_trait]
impl RaftLogReader<TypeConfig> for Arc<RaftStore> {
    async fn get_log_state(&mut self) -> StorageResult<LogState<TypeConfig>> {
        let last_log_id = self.get_last_log_entry().await?;
        let last_purged_log_id = self.get_last_purged_log_id().await?;
        let last_log_id = last_log_id.or(last_purged_log_id);

        Ok(LogState {
            last_purged_log_id,
            last_log_id,
        })
    }

    async fn try_get_log_entries<RB>(
        &mut self,
        range: RB
    ) -> StorageResult<Vec<Entry<TypeConfig>>>
    where
        RB: RangeBounds<u64> + Clone + Debug + Send + Sync,
    {
        let query = logs_query(range);
        let entries = self.log
            .fetch_all::<_, (Vec<u8>,)>(query, ())
            .await
            .map_err(log_read_error)?;

        entries.into_iter()
            .map(|(data,)| {
                rmp_serde::from_slice(&data)
                    .map_err(log_read_error)
            })
            .collect::<Result<Vec<_>, StorageError<NodeId>>>()
    }
}

#[async_trait::async_trait]
impl RaftSnapshotBuilder<TypeConfig, Cursor<Vec<u8>>> for Arc<RaftStore> {
    async fn build_snapshot(&mut self) -> StorageResult<Snapshot<NodeId, BasicNode, Cursor<Vec<u8>>>> {
        let last_applied_log = self.state_machine.last_applied_log();
        let last_membership = self.state_machine.last_membership();

        let snapshot_idx = self.get_and_inc_snapshot_index().await?;

        let snapshot_id = if let Some(last) = last_applied_log {
            format!("{}-{}-{}", last.leader_id, last.index, snapshot_idx)
        } else {
            format!("--{}", snapshot_idx)
        };

        let meta = SnapshotMeta {
            last_log_id: last_applied_log,
            last_membership,
            snapshot_id,
        };

        let data = create_snapshot(&self.snapshot_handle, &self.state_machine.data, &meta)
            .await
            .map_err(create_snapshot_err)?;

        Ok(Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(data.to_vec()))
        })
    }
}

#[async_trait::async_trait]
impl RaftStorage<TypeConfig> for Arc<RaftStore> {
    type SnapshotData = Cursor<Vec<u8>>;
    type LogReader = Self;
    type SnapshotBuilder = Self;

    async fn save_vote(&mut self, vote: &Vote<NodeId>) -> StorageResult<()> {
        self.set_vote(vote).await
    }

    async fn read_vote(&mut self) -> StorageResult<Option<Vote<NodeId>>> {
        self.get_vote().await
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }

    async fn append_to_log(&mut self, entries: &[&Entry<TypeConfig>]) -> StorageResult<()> {
        let qry = "INSERT INTO replicat_raft_logs (node_id, log_index, term, entry) VALUES (?, ?, ?, ?);";

        for entry in entries {
            let entry_data = rmp_serde::to_vec(entry).map_err(log_write_error)?;
            let node_id = entry.log_id.leader_id.node_id;
            let term = entry.log_id.leader_id.term;
            let log_index = entry.log_id.index;

            let params = (node_id, term, log_index, entry_data);

            self.log
                .execute(qry, params)
                .await
                .map_err(log_write_error)?;
        }

        Ok(())
    }

    async fn delete_conflict_logs_since(&mut self, log_id: LogId<NodeId>) -> StorageResult<()> {
        let qry = "DELETE FROM replicat_raft_logs WHERE log_index >= ?";

        self.log
            .execute(qry, (log_id.index,))
            .await
            .map_err(log_write_error)?;

        Ok(())
    }

    async fn purge_logs_upto(&mut self, log_id: LogId<NodeId>) -> StorageResult<()> {
        let qry = "DELETE FROM replicat_raft_logs WHERE log_index <= ?";

        self.log
            .execute(qry, (log_id.index,))
            .await
            .map_err(log_write_error)?;

        Ok(())
    }

    async fn last_applied_state(&mut self) -> StorageResult<(Option<LogId<NodeId>>, EffectiveMembership<NodeId, BasicNode>)> {
        Ok((
            self.state_machine.last_applied_log(),
            self.state_machine.last_membership(),
        ))
    }

    async fn apply_to_state_machine(&mut self, entries: &[&Entry<TypeConfig>]) -> StorageResult<Vec<Response>> {
        let mut res = Vec::with_capacity(entries.len());

        for entry in entries {
            self.state_machine.set_last_applied_log(entry.log_id).await?;

            match entry.payload {
                EntryPayload::Blank => res.push(Response::default()),
                EntryPayload::Normal(ref req) => match req {
                    Request::Execute { sql, value } => {
                        let params = rusqlite::params_from_iter(value.clone());

                        self.state_machine
                            .data
                            .execute(sql, params)
                            .await
                            .map_err(store_write_error)?;

                        res.push(Response::default());
                    }
                },
                EntryPayload::Membership(ref mem) => {
                    let membership =  EffectiveMembership::new(Some(entry.log_id), mem.clone());
                    self.state_machine.set_last_membership(membership).await?;
                    res.push(Response::default())
                }
            };
        }

        Ok(res)
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }

    async fn begin_receiving_snapshot(&mut self) -> StorageResult<Box<Self::SnapshotData>> {
        Ok(Box::new(Cursor::new(Vec::new())))
    }

    async fn install_snapshot(&mut self, meta: &SnapshotMeta<NodeId, BasicNode>, snapshot: Box<Self::SnapshotData>) -> StorageResult<()> {
        let mem = SqliteMemory::from_slice(&snapshot.into_inner())
            .map_err(create_snapshot_err)?;

        self.snapshot_handle
            .load_from_serialized(mem)
            .await
            .map_err(create_snapshot_err)?;

        write_snapshot_meta(&self.snapshot_handle, meta)
            .await
            .map_err(create_snapshot_err)?;

        Ok(())
    }

    async fn get_current_snapshot(&mut self) -> StorageResult<Option<Snapshot<NodeId, BasicNode, Self::SnapshotData>>> {
        let metadata = get_snapshot_meta(&self.snapshot_handle)
            .await
            .map_err(create_snapshot_err)?;

        let meta = match metadata {
            None => return Ok(None),
            Some(meta) => meta,
        };

        let data = self.snapshot_handle
            .serialize()
            .await
            .map_err(create_snapshot_err)?
            .map(|v| v.to_vec())
            .unwrap_or_default();

        Ok(Some(Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(data)),
        }))
    }
}

fn logs_query<RB>(range: RB) -> String
where
    RB: RangeBounds<u64>
{
    let start = match range.start_bound() {
        std::ops::Bound::Included(x) => *x,
        std::ops::Bound::Excluded(x) => *x + 1,
        std::ops::Bound::Unbounded => 0,
    };

    let end = match range.end_bound() {
        std::ops::Bound::Included(x) => *x,
        std::ops::Bound::Excluded(x) => *x - 1,
        std::ops::Bound::Unbounded => i64::MAX as u64,
    };

    format!("SELECT entry FROM replicat_raft_logs WHERE log_index >= {} AND log_index <= {};", start, end)
}

fn log_read_error(e: impl Error + 'static) -> StorageError<NodeId> {
    StorageError::IO {
        source: StorageIOError::new(ErrorSubject::Logs, ErrorVerb::Read, AnyError::new(&e)),
    }
}

fn log_write_error(e: impl Error + 'static) -> StorageError<NodeId> {
    StorageError::IO {
        source: StorageIOError::new(ErrorSubject::Logs, ErrorVerb::Write, AnyError::new(&e)),
    }
}

fn store_read_error(e: impl Error + 'static) -> StorageError<NodeId> {
    StorageError::IO {
        source: StorageIOError::new(ErrorSubject::Store, ErrorVerb::Read, AnyError::new(&e)),
    }
}

fn store_write_error(e: impl Error + 'static) -> StorageError<NodeId> {
    StorageError::IO {
        source: StorageIOError::new(ErrorSubject::Store, ErrorVerb::Write, AnyError::new(&e)),
    }
}

fn create_snapshot_err(e: impl Display) -> StorageError<NodeId> {
    StorageError::IO {
        source: StorageIOError::new(ErrorSubject::None, ErrorVerb::Write, AnyError::error(e)),
    }
}

async fn create_state_machine(data: StorageHandle) -> Result<StateMachine, anyhow::Error> {
    let last_applied_log = data
        .fetch_one::<_, (Vec<u8>,)>(REPLICAT_KV_SELECT_VALUE, (LAST_LOG_KEY,))
        .await?
        .map(|data| rmp_serde::from_slice(&data.0))
        .transpose()?;

    let last_membership = data
        .fetch_one::<_, (Vec<u8>,)>(REPLICAT_KV_SELECT_VALUE, (LAST_MEMBERSHIP_KEY,))
        .await?
        .map(|data| rmp_serde::from_slice(&data.0))
        .transpose()?;

    Ok(StateMachine {
        last_applied_log: RwLock::new(last_applied_log),
        last_membership: RwLock::new(last_membership.unwrap_or_default()),
        data
    })
}

async fn setup_log_store(conn: &StorageHandle) -> rusqlite::Result<()> {
    conn.fetch_one::<_, (String,)>("pragma journal_mode = WAL;", ()).await?;
    conn.execute("pragma synchronous = normal;", ()).await?;
    conn.execute("pragma temp_store = memory;", ()).await?;

    conn.execute(REPLICAT_RAFT_LOGS_TABLE, ()).await?;
    conn.execute(REPLICAT_KV_TABLE, ()).await?;

    Ok(())
}

async fn setup_snapshot_store(conn: &StorageHandle) -> rusqlite::Result<()> {
    conn.fetch_one::<_, (String,)>("pragma journal_mode = WAL;", ()).await?;
    conn.execute("pragma synchronous = normal;", ()).await?;
    conn.execute("pragma temp_store = memory;", ()).await?;

    conn.execute(REPLICAT_KV_TABLE, ()).await?;

    Ok(())
}