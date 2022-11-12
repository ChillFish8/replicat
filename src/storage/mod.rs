use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::path::Path;
use std::ptr;
use std::ptr::NonNull;

use flume::{self, Receiver, Sender};
use futures::channel::oneshot;
use rusqlite::{ffi, Connection, OptionalExtension, Params, Row};

type Task = Box<dyn FnOnce(&mut Connection) + Send + 'static>;

const CAPACITY: usize = 10;

#[derive(Debug, Clone)]
pub struct StorageHandle {
    tx: Sender<Task>,
}

impl StorageHandle {
    /// Connects to the SQLite database.
    ///
    /// This spawns 1 background threads with actions being executed within that thread.
    ///
    /// This approach reduces the affect of writes blocking reads and vice-versa.
    pub async fn open(path: impl AsRef<Path>) -> rusqlite::Result<Self> {
        let tx = setup_database(path).await?;
        Ok(Self { tx })
    }

    /// Connects to a new in-memory SQLite database.
    pub async fn open_in_memory() -> rusqlite::Result<Self> {
        Self::open(":memory:").await
    }

    /// Creates a storage handle from a existing tasks TX.
    fn from_tasks(tx: Sender<Task>) -> Self {
        Self { tx }
    }

    /// Execute a SQL statement with some provided parameters.
    pub async fn execute<P>(
        &self,
        sql: impl AsRef<str>,
        params: P,
    ) -> rusqlite::Result<usize>
    where
        P: Params + Clone + Send + 'static,
    {
        let sql = sql.as_ref().to_string();
        self.submit_task(move |conn| {
            let mut prepared = conn.prepare_cached(&sql)?;
            prepared.execute(params.clone())
        })
        .await
    }

    /// Fetch a single row from a given SQL statement with some provided parameters.
    pub async fn fetch_one<P, T>(
        &self,
        sql: impl AsRef<str>,
        params: P,
    ) -> rusqlite::Result<Option<T>>
    where
        P: Params + Send + 'static,
        T: FromRow + Send + 'static,
    {
        let sql = sql.as_ref().to_string();

        self.submit_task(move |conn| {
            let mut prepared = conn.prepare_cached(&sql)?;
            prepared.query_row(params, T::from_row).optional()
        })
        .await
    }

    /// Fetch a all rows from a given SQL statement with some provided parameters.
    pub async fn fetch_all<P, T>(
        &self,
        sql: impl AsRef<str>,
        params: P,
    ) -> rusqlite::Result<Vec<T>>
    where
        P: Params + Send + 'static,
        T: FromRow + Send + 'static,
    {
        let sql = sql.as_ref().to_string();

        self.submit_task(move |conn| {
            let mut prepared = conn.prepare_cached(&sql)?;
            let mut iter = prepared.query(params)?;

            let mut rows = Vec::with_capacity(4);
            while let Some(row) = iter.next()? {
                rows.push(T::from_row(row)?);
            }

            Ok(rows)
        })
        .await
    }

    /// Submits a writer task to execute.
    ///
    /// This executes the callback on the memory view connection which should be
    /// significantly faster to modify or read.
    async fn submit_task<CB, T>(&self, inner: CB) -> rusqlite::Result<T>
    where
        T: Send + 'static,
        CB: FnOnce(&mut Connection) -> rusqlite::Result<T> + Send + 'static,
    {
        let (tx, rx) = oneshot::channel();

        let cb = move |conn: &mut Connection| {
            let res = inner(conn);
            let _ = tx.send(res);
        };

        self.tx
            .send_async(Box::new(cb))
            .await
            .expect("send message");

        rx.await.unwrap()
    }

    /// Creates a copy of the existing database producing a new in-memory SQLite database.
    pub async fn create_memory_copy(&self) -> rusqlite::Result<Self> {
        let conn = self.submit_task(|conn| create_memory_view(conn)).await?;
        let (tx, tasks) = flume::bounded(CAPACITY);

        std::thread::spawn(move || run_tasks(conn, tasks));

        Ok(Self::from_tasks(tx))
    }

    pub async fn serialize(&self) -> rusqlite::Result<SqliteMemory> {
        self.submit_task(|conn| serialize_db(conn)).await
    }
}

/// A helper trait for converting between a Row reference and the given type.
///
/// This is required due to the nature of rows being tied to the database connection
/// which cannot be shared outside of the thread the actor runs in.
pub trait FromRow: Sized {
    fn from_row(row: &Row) -> rusqlite::Result<Self>;
}

async fn setup_database(path: impl AsRef<Path>) -> rusqlite::Result<Sender<Task>> {
    let path = path.as_ref().to_path_buf();
    let (tx, rx) = flume::bounded(CAPACITY);

    tokio::task::spawn_blocking(move || setup_disk_handle(&path, rx))
        .await
        .expect("spawn background runner")?;

    Ok(tx)
}

fn setup_disk_handle(path: &Path, tasks: Receiver<Task>) -> rusqlite::Result<()> {
    let disk = Connection::open(path)?;
    std::thread::spawn(move || run_tasks(disk, tasks));

    Ok(())
}

/// Runs all tasks received with a mutable reference to the given connection.
fn run_tasks(mut conn: Connection, tasks: Receiver<Task>) {
    while let Ok(task) = tasks.recv() {
        (task)(&mut conn);
    }
}

#[derive(Debug)]
pub struct SqliteMemory {
    ptr: NonNull<u8>,
    size: usize,
}

impl SqliteMemory {
    /// Creates a new [SqliteMemory] from a given slice.
    ///
    /// This method copies the data from the provided slice to the internal memory.
    pub fn from_slice(buf: &[u8]) -> rusqlite::Result<Self> {
        unsafe {
            let size = buf.len();
            let ptr: *mut c_void = ffi::sqlite3_malloc64(size as u64);

            if ptr.is_null() {
                return Err(no_memory());
            }

            ptr::copy_nonoverlapping(buf.as_ptr(), ptr.cast(), size);

            Ok(Self::from_raw(ptr.cast(), size))
        }
    }

    /// Constructs [SqliteMemory] from a provided pointer and size.
    ///
    /// SAFETY:
    /// This is UB if the ptr is not the only reference to the data itself
    /// as the memory will be freed when the struct is dropped.
    ///
    /// This must also be allocated from SQLite's `sqlite3_malloc64` otherwise
    /// this is UB.
    unsafe fn from_raw(ptr: *mut u8, size: usize) -> Self {
        SqliteMemory {
            ptr: NonNull::new(ptr).unwrap(),
            size,
        }
    }
}

// SAFETY: SqliteMemory both owns
unsafe impl Send for SqliteMemory {}
unsafe impl Sync for SqliteMemory {}

impl Deref for SqliteMemory {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr.as_ptr(), self.size) }
    }
}

impl Drop for SqliteMemory {
    fn drop(&mut self) {
        unsafe { ffi::sqlite3_free(self.ptr.as_ptr().cast()) }
    }
}

fn create_memory_view(disk: &Connection) -> rusqlite::Result<Connection> {
    let mut view = Connection::open_in_memory()?;

    // We're temporary and in memory, we can disable essentially all durability settings.
    view.query_row("pragma journal_mode = OFF;", (), |_r| Ok(()))?;
    view.execute("pragma synchronous = OFF;", ())?;
    view.execute("pragma temp_store = memory;", ())?;

    // Created a deserialized view of the existing database.
    let data = serialize_db(disk)?;
    deserialize_db(&mut view, data)?;

    Ok(view)
}

fn serialize_db(conn: &Connection) -> rusqlite::Result<SqliteMemory> {
    unsafe {
        let handle = conn.handle();

        let mut size = 0i64;
        let size_ptr = &mut size as *mut i64;
        let ptr: *mut u8 = ffi::sqlite3_serialize(handle, ptr::null(), size_ptr, 0);

        if ptr.is_null() || size == -1 {
            // It doesn't matter if `ptr` is null or not.
            ffi::sqlite3_free(ptr.cast());
            return Err(no_memory());
        }

        Ok(SqliteMemory::from_raw(ptr, size as usize))
    }
}

fn deserialize_db(
    write_to: &mut Connection,
    data: SqliteMemory,
) -> rusqlite::Result<()> {
    let data = ManuallyDrop::new(data);

    unsafe {
        let handle = write_to.handle();

        // WARNING:
        //  Do not call `sqlite_free` on the serialized buffer past this point.
        //  The database will directly use this buffer and de-alloc it once closed.
        let status = ffi::sqlite3_deserialize(
            handle,
            ptr::null(),
            data.ptr.as_ptr(),
            data.size as i64,
            data.size as i64,
            (ffi::SQLITE_DESERIALIZE_FREEONCLOSE | ffi::SQLITE_DESERIALIZE_RESIZEABLE)
                as u32,
        );

        if status != ffi::SQLITE_OK {
            return Err(rusqlite::Error::SqliteFailure(
                ffi::Error::new(status),
                Some("Unable to create memory view for database".to_string()),
            ));
        }
    }

    Ok(())
}

fn no_memory() -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(
        ffi::Error::new(ffi::SQLITE_NOMEM),
        Some("Failed to allocate buffer, is the system out of memory?".to_string()),
    )
}

#[cfg(test)]
mod tests {
    use std::env::temp_dir;

    use super::*;

    fn create_test_connection() -> Connection {
        let conn = Connection::open_in_memory().expect("open db in memory");
        conn.execute(
            "CREATE TABLE IF NOT EXISTS person (
                id    INTEGER PRIMARY KEY,
                name  TEXT NOT NULL,
                data  BLOB
            )",
            (), // empty list of parameters.
        )
        .expect("create table");
        conn.execute(
            "INSERT INTO person (id, name, data) VALUES (1, 'cf8', 'tada')",
            (),
        )
        .expect("insert person");

        conn
    }

    #[test]
    fn test_serialize_db() {
        let conn = create_test_connection();
        let res = serialize_db(&conn);
        assert!(
            res.is_ok(),
            "Expected serialization to pass. Res: {:?}",
            res
        );
    }

    #[test]
    fn test_deserialize_db() {
        let conn = create_test_connection();
        let data = serialize_db(&conn).expect("Serialize database");
        drop(conn);

        let data_copy = data.to_vec();

        // Try deserialize using just the memory we get from sqlite.
        let mut write_to = Connection::open_in_memory().unwrap();
        let res = deserialize_db(&mut write_to, data);
        assert!(
            res.is_ok(),
            "Expected raw memory deserialization to pass. Res: {:?}",
            res
        );

        // Try deserialize using the memory
        let data = SqliteMemory::from_slice(&data_copy).expect("Create memory.");
        let mut write_to = Connection::open_in_memory().unwrap();
        let res = deserialize_db(&mut write_to, data);
        assert!(
            res.is_ok(),
            "Expected owned memory deserialization to pass. Res: {:?}",
            res
        );

        let (id, name, data) = write_to
            .query_row(
                "SELECT id, name, data FROM person WHERE id = 1;",
                (),
                |row| {
                    Ok((
                        row.get::<_, i32>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .expect("query database");

        assert_eq!(id, 1, "Ids should be the same");
        assert_eq!(name, "cf8".to_string(), "Name should be the same");
        assert_eq!(data, "tada", "Data should be the same");
    }

    #[test]
    fn test_create_memory_view() {
        let conn = Connection::open_in_memory().expect("open db in memory");

        conn.execute(
            "CREATE TABLE person (
                id    INTEGER PRIMARY KEY,
                name  TEXT NOT NULL,
                data  BLOB
            )",
            (), // empty list of parameters.
        )
        .expect("create table");

        conn.execute(
            "INSERT INTO person (id, name, data) VALUES (1, 'cf8', 'tada')",
            (),
        )
        .expect("insert person");

        let view = create_memory_view(&conn).expect("create memory view");

        let (id, name, data) = view
            .query_row(
                "SELECT id, name, data FROM person WHERE id = 1;",
                (),
                |row| {
                    Ok((
                        row.get::<_, i32>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .expect("query database");

        assert_eq!(id, 1, "Ids should be the same");
        assert_eq!(name, "cf8".to_string(), "Name should be the same");
        assert_eq!(data, "tada", "Data should be the same");
    }

    #[test]
    fn test_setup_memory_view_handle_blank_db_resize() {
        let disk = Connection::open_in_memory().expect("open db in memory");

        disk.execute(
            "CREATE TABLE person (
                id    INTEGER PRIMARY KEY,
                name  TEXT NOT NULL,
                data  BLOB
            )",
            (), // empty list of parameters.
        )
        .expect("create table");

        disk.execute(
            "INSERT INTO person (id, name, data) VALUES (1, 'cf8', 'tada')",
            (),
        )
        .expect("insert person");

        let view = create_memory_view(&disk).expect("create memory view");
        view.execute(
            "INSERT INTO person (id, name, data) VALUES (2, 'cf9', 'tada')",
            (),
        )
        .expect("Add and resize");
        view.execute(
            "INSERT INTO person (id, name, data) VALUES (3, 'cf10', 'tada')",
            (),
        )
        .expect("Add and resize");
        view.execute(
            "INSERT INTO person (id, name, data) VALUES (4, 'cf11', 'tada')",
            (),
        )
        .expect("Add and resize");
    }

    #[tokio::test]
    async fn test_memory_storage_handle() {
        let handle = StorageHandle::open_in_memory().await.expect("open DB");

        run_storage_handle_suite(handle).await;
    }

    #[tokio::test]
    async fn test_disk_storage_handle() {
        let path = temp_dir().join(uuid::Uuid::new_v4().to_string());
        let handle = StorageHandle::open(path).await.expect("open DB");

        run_storage_handle_suite(handle).await;
    }

    #[derive(Debug, Eq, PartialEq)]
    struct Person {
        id: i32,
        name: String,
        data: String,
    }

    impl FromRow for Person {
        fn from_row(row: &Row) -> rusqlite::Result<Self> {
            Ok(Self {
                id: row.get(0)?,
                name: row.get(1)?,
                data: row.get(2)?,
            })
        }
    }

    async fn run_storage_handle_suite(handle: StorageHandle) {
        handle
            .execute(
                "CREATE TABLE person (
                    id    INTEGER PRIMARY KEY,
                    name  TEXT NOT NULL,
                    data  BLOB
                )",
                (), // empty list of parameters.
            )
            .await
            .expect("create table");

        let res = handle
            .fetch_one::<_, Person>("SELECT id, name, data FROM person;", ())
            .await
            .expect("execute statement");
        assert!(res.is_none(), "Expected no rows to be returned.");

        handle
            .execute(
                "INSERT INTO person (id, name, data) VALUES (1, 'cf8', 'tada');",
                (),
            )
            .await
            .expect("Insert row");

        let res = handle
            .fetch_one::<_, Person>("SELECT id, name, data FROM person;", ())
            .await
            .expect("execute statement");
        assert_eq!(
            res,
            Some(Person {
                id: 1,
                name: "cf8".to_string(),
                data: "tada".to_string()
            }),
        );

        handle
            .execute(
                "INSERT INTO person (id, name, data) VALUES (2, 'cf6', 'tada2');",
                (),
            )
            .await
            .expect("Insert row");

        let res = handle
            .fetch_all::<_, Person>(
                "SELECT id, name, data FROM person ORDER BY id ASC;",
                (),
            )
            .await
            .expect("execute statement");
        assert_eq!(
            res,
            vec![
                Person {
                    id: 1,
                    name: "cf8".to_string(),
                    data: "tada".to_string()
                },
                Person {
                    id: 2,
                    name: "cf6".to_string(),
                    data: "tada2".to_string()
                },
            ],
        );
    }
}
