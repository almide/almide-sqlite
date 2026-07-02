// Native Rust implementation for almide-sqlite.
//
// Uses a global handle registry to map Int handles to rusqlite::Connection.
// Thread-safe via Mutex. Handles are monotonically increasing i64.

use rusqlite::{params_from_iter, Connection};
use std::collections::HashMap;
use std::sync::Mutex;

// The generated crate's Map representation (insertion-ordered Vec<(K,V)>).
// Almide `Map[String, String]` crosses the FFI as this type, not std HashMap.
use crate::AlmideMap;

static REGISTRY: Mutex<Option<HandleRegistry>> = Mutex::new(None);

struct HandleRegistry {
    connections: HashMap<i64, Connection>,
    next_id: i64,
}

impl HandleRegistry {
    fn new() -> Self {
        Self {
            connections: HashMap::new(),
            next_id: 1,
        }
    }
}

fn with_registry<T>(f: impl FnOnce(&mut HandleRegistry) -> T) -> T {
    let mut guard = REGISTRY.lock().unwrap();
    let reg = guard.get_or_insert_with(HandleRegistry::new);
    f(reg)
}

fn with_conn<T>(handle: i64, f: impl FnOnce(&Connection) -> Result<T, String>) -> Result<T, String> {
    let guard = REGISTRY.lock().unwrap();
    let reg = guard.as_ref().ok_or("sqlite: no connections")?;
    let conn = reg
        .connections
        .get(&handle)
        .ok_or_else(|| format!("sqlite: invalid handle {handle}"))?;
    f(conn)
}

// ── Open / Close ──

pub fn native_open(path: &str) -> Result<i64, String> {
    let conn = Connection::open(path).map_err(|e| format!("sqlite open: {e}"))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| format!("sqlite pragma: {e}"))?;
    let id = with_registry(|reg| {
        let id = reg.next_id;
        reg.next_id += 1;
        reg.connections.insert(id, conn);
        id
    });
    Ok(id)
}

pub fn native_close(handle: i64) -> Result<(), String> {
    with_registry(|reg| {
        reg.connections
            .remove(&handle)
            .ok_or_else(|| format!("sqlite: invalid handle {handle}"))?;
        Ok(())
    })
}

// ── Execute ──

pub fn native_exec(handle: i64, sql: &str) -> Result<(), String> {
    with_conn(handle, |conn| {
        conn.execute_batch(sql)
            .map_err(|e| format!("sqlite exec: {e}"))
    })
}

pub fn native_exec_params(handle: i64, sql: &str, params: &[String]) -> Result<(), String> {
    with_conn(handle, |conn| {
        conn.execute(sql, params_from_iter(params.iter()))
            .map(|_| ())
            .map_err(|e| format!("sqlite exec_params: {e}"))
    })
}

// ── Query ──

pub fn native_query(handle: i64, sql: &str) -> Result<Vec<AlmideMap<String, String>>, String> {
    query_impl(handle, sql, &[])
}

pub fn native_query_params(
    handle: i64,
    sql: &str,
    params: &[String],
) -> Result<Vec<AlmideMap<String, String>>, String> {
    query_impl(handle, sql, params)
}

fn query_impl(
    handle: i64,
    sql: &str,
    params: &[String],
) -> Result<Vec<AlmideMap<String, String>>, String> {
    with_conn(handle, |conn| {
        let mut stmt = conn.prepare(sql).map_err(|e| format!("sqlite prepare: {e}"))?;
        let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

        let rows = stmt
            .query_map(params_from_iter(params.iter()), |row| {
                let mut map = AlmideMap::new();
                for (i, name) in col_names.iter().enumerate() {
                    let val: String = row
                        .get::<_, rusqlite::types::Value>(i)
                        .map(|v| val_to_string(&v))
                        .unwrap_or_default();
                    map.insert(name.clone(), val);
                }
                Ok(map)
            })
            .map_err(|e| format!("sqlite query: {e}"))?;

        rows.into_iter()
            .map(|r| r.map_err(|e| format!("sqlite row: {e}")))
            .collect()
    })
}

// ── Metadata ──

pub fn native_changes(handle: i64) -> Result<i64, String> {
    with_conn(handle, |conn| Ok(conn.changes() as i64))
}

pub fn native_last_insert_id(handle: i64) -> Result<i64, String> {
    with_conn(handle, |conn| Ok(conn.last_insert_rowid()))
}

// ── Table info ──

pub fn native_tables(handle: i64) -> Result<Vec<String>, String> {
    let rows = query_impl(
        handle,
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
        &[],
    )?;
    Ok(rows.into_iter().filter_map(|r| r.get(&"name".to_string()).cloned()).collect())
}

pub fn native_table_exists(handle: i64, name: &str) -> Result<bool, String> {
    let rows = query_impl(
        handle,
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1",
        &[name.to_string()],
    )?;
    Ok(!rows.is_empty())
}

// ── Helpers ──

fn val_to_string(v: &rusqlite::types::Value) -> String {
    match v {
        rusqlite::types::Value::Null => String::new(),
        rusqlite::types::Value::Integer(i) => i.to_string(),
        rusqlite::types::Value::Real(f) => f.to_string(),
        rusqlite::types::Value::Text(s) => s.clone(),
        rusqlite::types::Value::Blob(b) => format!("<blob {} bytes>", b.len()),
    }
}
