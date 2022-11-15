use std::borrow::Cow;
use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::Result;
use rusqlite::Params;
use tonic::Status;

use crate::{FromRow, NodeId, ReplicatNetwork, rpc, StorageHandle};
use crate::rpc::RpcClient;
use crate::storage::params::IntoTransportableParams;

#[derive(Clone)]
pub struct ReplicatClient {
    /// The leader node to send request to.
    ///
    /// All traffic should be sent to the leader in a cluster.
    leader: Arc<RwLock<(NodeId, Cow<'static, str>)>>,

    /// The existing network of nodes.
    network: Arc<ReplicatNetwork>,

    /// The local state of the system which can be read from.
    local_data: StorageHandle,
}

impl ReplicatClient {
    /// Creates a new client with a given leader node.
    pub(crate) fn new(
        leader_id: NodeId,
        leader_addr: String,
        network: Arc<ReplicatNetwork>,
        local_data: StorageHandle,
    ) -> Self {
        Self {
            leader: Arc::new(RwLock::new((leader_id, Cow::Owned(leader_addr)))),
            network,
            local_data,
        }
    }

    fn get_leader_client(&self) -> Result<RpcClient, rpc::Error> {
        let (node_id, addr) = {
            self.leader.read().clone()
        };

        self.network
            .get_or_connect_client(node_id, &addr)
            .map_err(|e| rpc::Error::NetworkError(Status::unavailable("Unable to connect to remote client")))
    }

    /// Fetch a single row from a given SQL statement with some provided parameters.
    pub async fn fetch_one<T, P>(&self, sql: impl AsRef<str>, params: P) -> rusqlite::Result<Option<T>>
    where
        T: FromRow + Send + 'static,
        P: Params + Send + 'static,
    {
        self.local_data.fetch_one(sql, params).await
    }

    /// Fetch a single row from a given SQL statement with some provided parameters.
    pub async fn fetch_all<T, P>(&self, sql: impl AsRef<str>, params: P) -> rusqlite::Result<Vec<T>>
    where
        T: FromRow + Send + 'static,
        P: Params + Send + 'static,
    {
        self.local_data.fetch_all(sql, params).await
    }

    /// Execute a SQL statement with some provided parameters.
    ///
    /// This is executed on the leader node, changes may not be immediately reflected.
    pub async fn execute<T, P>(&self, sql: impl Into<String>, params: P) -> Result<(), rpc::Error>
    where
        T: FromRow + Send + 'static,
        P: IntoTransportableParams,
    {
        let sql = sql.into();
        let transportable = params.to_params();

        let mut n_retry = 3;
        loop {
            let mut client = self.get_leader_client()?;

            let result = client.mutate_state(sql.clone(), &transportable).await;

            match result {
                Err(rpc::Error::UpdateLeader(leader)) => {
                    {
                        let mut lock = self.leader.write();
                        (*lock) = (leader.id, Cow::Owned(leader.rpc_addr));
                    }

                    n_retry -= 1;

                    if n_retry == 0 {
                        return Err(rpc::Error::UnknownLeader);
                    }
                },
                Err(e) => return Err(e),
                Ok(()) => return Ok(()),
            }
        }
    }
}
