fn main() {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    match conn.execute_batch("CREATE VIRTUAL TABLE t USING fts5(a, b); INSERT INTO t VALUES ('hello world', 'x');") {
        Ok(_) => {
            let n: i64 = conn.query_row("SELECT COUNT(*) FROM t WHERE t MATCH 'hello'", [], |r| r.get(0)).unwrap();
            println!("FTS5 OK, matches={n}");
        }
        Err(e) => println!("FTS5 FAIL: {e}"),
    }
}
