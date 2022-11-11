mod storage;

use std::ptr;
use anyhow::Result;
use rusqlite::{Connection, OpenFlags};
use rusqlite::ffi;


fn main() -> Result<()> {
    let conn = Connection::open_in_memory()?;
    conn.execute("CREATE TABLE testing (name TEXT);", ())?;
    conn.execute("INSERT INTO testing (name) VALUES (?)", ("hello",))?;

    unsafe {
        let handle = conn.handle();

        let mut size = 0i64;
        let ptr = &mut size as *mut i64;
        let mem = ffi::sqlite3_serialize(handle, ptr::null(), ptr, 0);
        dbg!(mem, mem.is_null(), size);

        let buff: &[u8] = std::slice::from_raw_parts(mem, size as usize);
        dbg!(buff);

        let conn2 = Connection::open_in_memory()?;
        let handle2 = conn2.handle();

        let status = ffi::sqlite3_deserialize(handle2, ptr::null(), mem, 0, size, 0);
        dbg!(status);

        ffi::sqlite3_free(mem as _);
        dbg!(buff);

        let mut stmt = conn.prepare("SELECT* FROM testing")?;
        let person_iter = stmt.query_map([], |row| {
            let name: String = row.get(0).unwrap();
            Ok(name)
        })?;

        for person in person_iter {
            println!("Found thing {:?}", person.unwrap());
        }
    }

    println!("we good??");

    Ok(())
}