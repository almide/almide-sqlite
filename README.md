# almide-sqlite

SQLite3 database access for Almide.

## Install

```bash
almide add almide-sqlite@v0.1.0
```

## Usage

```almide
import almide_sqlite as db

effect fn main() -> Unit = {
  let conn = db.open("app.db")!

  db.exec(conn, "CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)")!
  db.exec_params(conn, "INSERT INTO users (name, age) VALUES (?, ?)", ["Alice", "30"])!
  db.exec_params(conn, "INSERT INTO users (name, age) VALUES (?, ?)", ["Bob", "25"])!

  let rows = db.query(conn, "SELECT * FROM users")!
  for row in rows {
    println("${db.pluck(row, "name")} is ${db.pluck(row, "age")} years old")
  }

  let names = db.query(conn, "SELECT name FROM users")! |> db.column("name")
  println("All names: ${list.join(names, ", ")}")

  db.close(conn)!
}
```

## API

### Open / Close

| Function | Description |
|----------|-------------|
| `db.open(path)` | Open (or create) a database file |
| `db.open_memory()` | Open an in-memory database |
| `db.close(db)` | Close a database connection |

### Execute

| Function | Description |
|----------|-------------|
| `db.exec(db, sql)` | Execute SQL with no result set |
| `db.exec_params(db, sql, params)` | Execute with `?` parameter binding |

### Query

| Function | Description |
|----------|-------------|
| `db.query(db, sql)` | Query, returns `List[Map[String, String]]` |
| `db.query_params(db, sql, params)` | Query with `?` parameter binding |

### Row Helpers

| Function | Description |
|----------|-------------|
| `db.first(rows)` | First row as `Option[Map[String, String]]` |
| `db.column(rows, name)` | Extract a single column as `List[String]` |
| `db.pluck(row, key)` | Get a value from a row, defaults to `""` |

### Metadata

| Function | Description |
|----------|-------------|
| `db.changes(db)` | Number of rows affected by last statement |
| `db.last_insert_id(db)` | Last inserted rowid |

### Transactions

| Function | Description |
|----------|-------------|
| `db.begin(db)` | Begin transaction |
| `db.commit(db)` | Commit transaction |
| `db.rollback(db)` | Rollback transaction |
| `db.transaction(db, fn)` | Execute fn inside a transaction (auto commit/rollback) |

### Table Info

| Function | Description |
|----------|-------------|
| `db.tables(db)` | List all table names |
| `db.table_exists(db, name)` | Check if a table exists |

## Notes

- All column values are returned as `String`. Use `int.parse()` / `float.parse()` for conversion.
- Parameters use `?` placeholders (SQLite positional binding).
- WAL journal mode and foreign keys are enabled by default.
- `Db` is an opaque `Int` handle. Always call `db.close()` when done.

## License

MIT
