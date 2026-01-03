use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, OnceLock};

use postgres::{types::Type as PgType, Client as PgClient, NoTls, Row as PgRow};
use redis::{Commands, Connection as RedisConnection};
use rusqlite::{types::ValueRef, Connection};

use crate::runtime::{
    ensure_arity, expect_int, expect_string, make_map_value, make_vec_value, option_none_value,
    option_some_value, result_err_value, result_ok_value, IntType, Interpreter, MapKey,
    ModuleValue, PrimitiveType, RuntimeError, RuntimeResult, StructInstance, TypeTag, Value,
};

static CONNECTIONS: OnceLock<Mutex<HashMap<i64, DbBackend>>> = OnceLock::new();
static NEXT_CONN_ID: AtomicI64 = AtomicI64::new(1);

pub fn forge_db_module() -> Value {
    let mut map = HashMap::new();
    map.insert("open".to_string(), Value::Builtin(db_open));
    map.insert("close".to_string(), Value::Builtin(db_close));
    map.insert("exec".to_string(), Value::Builtin(db_exec));
    map.insert("query".to_string(), Value::Builtin(db_query));
    map.insert("begin".to_string(), Value::Builtin(db_begin_tx));
    map.insert("commit".to_string(), Value::Builtin(db_commit_tx));
    map.insert("rollback".to_string(), Value::Builtin(db_rollback_tx));
    map.insert("get".to_string(), Value::Builtin(db_get));
    map.insert("set".to_string(), Value::Builtin(db_set));
    map.insert("del".to_string(), Value::Builtin(db_del));

    Value::Module(ModuleValue {
        name: "db".to_string(),
        fields: map,
    })
}

enum DbBackend {
    Sqlite(SqliteConn),
    Postgres(PostgresConn),
    Redis(RedisConn),
}

struct SqliteConn {
    conn: Connection,
}

struct PostgresConn {
    client: PgClient,
}

struct RedisConn {
    conn: RedisConnection,
}

fn connections() -> &'static Mutex<HashMap<i64, DbBackend>> {
    CONNECTIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn next_conn_id() -> i64 {
    NEXT_CONN_ID.fetch_add(1, Ordering::SeqCst)
}

fn conn_tag() -> TypeTag {
    TypeTag::Struct {
        name: "db::Connection".to_string(),
        params: Vec::new(),
    }
}

fn exec_tag() -> TypeTag {
    TypeTag::Struct {
        name: "db::ExecResult".to_string(),
        params: Vec::new(),
    }
}

fn option_string_tag() -> TypeTag {
    TypeTag::Option(Box::new(TypeTag::Primitive(PrimitiveType::String)))
}

fn sql_rows_tag() -> TypeTag {
    TypeTag::Vec(Box::new(TypeTag::Map(
        Box::new(TypeTag::Primitive(PrimitiveType::String)),
        Box::new(TypeTag::Unknown),
    )))
}

fn wrap_ok(value: Value, ok: Option<TypeTag>) -> RuntimeResult<Value> {
    Ok(result_ok_value(
        value,
        ok,
        Some(TypeTag::Primitive(PrimitiveType::String)),
    ))
}

fn wrap_err(msg: String, ok: Option<TypeTag>) -> RuntimeResult<Value> {
    Ok(result_err_value(
        Value::String(msg),
        ok,
        Some(TypeTag::Primitive(PrimitiveType::String)),
    ))
}

fn expect_conn_id(handle: &Value) -> RuntimeResult<i64> {
    if let Value::Struct(st) = handle {
        if let Some(id) = st.fields.get("id") {
            let raw = expect_int(id)?;
            return i64::try_from(raw)
                .map_err(|_| RuntimeError::new("db: connection id out of range"));
        }
    }
    Err(RuntimeError::new(
        "db: expected db::Connection with field `id`",
    ))
}

fn build_conn_struct(id: i64) -> Value {
    let mut fields = HashMap::new();
    fields.insert("id".to_string(), Value::Int(id.into()));
    Value::Struct(StructInstance {
        name: Some("db::Connection".to_string()),
        type_params: Vec::new(),
        fields,
    })
}

fn build_exec_struct(rows: i64) -> Value {
    let mut fields = HashMap::new();
    fields.insert("rows_affected".to_string(), Value::Int(rows.into()));
    Value::Struct(StructInstance {
        name: Some("db::ExecResult".to_string()),
        type_params: Vec::new(),
        fields,
    })
}

fn db_open(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "db.open")?;
        let backend = expect_string(&args[0])?;
        let target = expect_string(&args[1])?;

        match backend.as_str() {
            "sqlite" => {
                let path = target.strip_prefix("sqlite:").unwrap_or(&target).to_string();
                match Connection::open(path) {
                    Ok(conn) => {
                        let id = next_conn_id();
                        connections()
                            .lock()
                            .unwrap()
                            .insert(id, DbBackend::Sqlite(SqliteConn { conn }));
                        wrap_ok(build_conn_struct(id), Some(conn_tag()))
                    }
                    Err(e) => wrap_err(e.to_string(), Some(conn_tag())),
                }
            }
            "postgres" => match PgClient::connect(target.as_str(), NoTls) {
                Ok(client) => {
                    let id = next_conn_id();
                    connections()
                        .lock()
                        .unwrap()
                        .insert(id, DbBackend::Postgres(PostgresConn { client }));
                    wrap_ok(build_conn_struct(id), Some(conn_tag()))
                }
                Err(e) => wrap_err(e.to_string(), Some(conn_tag())),
            },
            "redis" => {
                match redis::Client::open(target.as_str()).and_then(|client| client.get_connection()) {
                    Ok(conn) => {
                        let id = next_conn_id();
                        connections()
                            .lock()
                            .unwrap()
                            .insert(id, DbBackend::Redis(RedisConn { conn }));
                        wrap_ok(build_conn_struct(id), Some(conn_tag()))
                    }
                    Err(e) => wrap_err(e.to_string(), Some(conn_tag())),
                }
            }
            other => wrap_err(
                format!("db.open: backend `{other}` is not supported"),
                Some(conn_tag()),
            ),
        }
    })
}

fn db_close(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "db.close")?;
        let id = expect_conn_id(&args[0])?;
        connections().lock().unwrap().remove(&id);
        wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new())))
    })
}

fn db_exec(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "db.exec")?;
        let id = expect_conn_id(&args[0])?;
        let sql = expect_string(&args[1])?;

        let mut guard = connections().lock().unwrap();
        let backend = guard
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("db.exec: invalid connection handle"))?;
        let rows = backend.exec(&sql)?;
        drop(guard);
        wrap_ok(build_exec_struct(rows), Some(exec_tag()))
    })
}

fn db_query(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "db.query")?;
        let id = expect_conn_id(&args[0])?;
        let sql = expect_string(&args[1])?;

        let mut guard = connections().lock().unwrap();
        let backend = guard
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("db.query: invalid connection handle"))?;
        let rows = backend.query(&sql)?;
        drop(guard);
        wrap_ok(rows, Some(sql_rows_tag()))
    })
}

fn db_begin_tx(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "db.begin")?;
        let id = expect_conn_id(&args[0])?;
        let mut guard = connections().lock().unwrap();
        let backend = guard
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("db.begin: invalid connection handle"))?;
        backend.begin_tx()?;
        drop(guard);
        wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new())))
    })
}

fn db_commit_tx(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "db.commit")?;
        let id = expect_conn_id(&args[0])?;
        let mut guard = connections().lock().unwrap();
        let backend = guard
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("db.commit: invalid connection handle"))?;
        backend.commit_tx()?;
        drop(guard);
        wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new())))
    })
}

fn db_rollback_tx(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 1, "db.rollback")?;
        let id = expect_conn_id(&args[0])?;
        let mut guard = connections().lock().unwrap();
        let backend = guard
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("db.rollback: invalid connection handle"))?;
        backend.rollback_tx()?;
        drop(guard);
        wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new())))
    })
}

fn db_get(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "db.get")?;
        let id = expect_conn_id(&args[0])?;
        let key = expect_string(&args[1])?;

        let mut guard = connections().lock().unwrap();
        let backend = guard
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("db.get: invalid connection handle"))?;
        let opt = backend.get(&key)?;
        drop(guard);
        wrap_ok(opt, Some(option_string_tag()))
    })
}

fn db_set(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 3, "db.set")?;
        let id = expect_conn_id(&args[0])?;
        let key = expect_string(&args[1])?;
        let value_str = args[2].to_string_value();

        let mut guard = connections().lock().unwrap();
        let backend = guard
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("db.set: invalid connection handle"))?;
        backend.set(&key, &value_str)?;
        drop(guard);
        wrap_ok(Value::Null, Some(TypeTag::Tuple(Vec::new())))
    })
}

fn db_del(_interp: &Interpreter, args: Vec<Value>) -> LocalBoxFuture<'static, RuntimeResult<Value>> {
    Box::pin(async move {
        ensure_arity(&args, 2, "db.del")?;
        let id = expect_conn_id(&args[0])?;
        let key = expect_string(&args[1])?;

        let mut guard = connections().lock().unwrap();
        let backend = guard
            .get_mut(&id)
            .ok_or_else(|| RuntimeError::new("db.del: invalid connection handle"))?;
        let deleted = backend.del(&key)?;
        drop(guard);
        wrap_ok(
            Value::Int(i128::from(deleted)),
            Some(TypeTag::Primitive(PrimitiveType::Int(IntType::I64))),
        )
    })
}

impl DbBackend {
    fn exec(&mut self, sql: &str) -> Result<i64, RuntimeError> {
        match self {
            DbBackend::Sqlite(conn) => conn.exec(sql),
            DbBackend::Postgres(conn) => conn.exec(sql),
            DbBackend::Redis(_) => Err(RuntimeError::new(
                "db.exec is unavailable for redis connections",
            )),
        }
    }

    fn query(&mut self, sql: &str) -> Result<Value, RuntimeError> {
        match self {
            DbBackend::Sqlite(conn) => conn.query(sql),
            DbBackend::Postgres(conn) => conn.query(sql),
            DbBackend::Redis(_) => Err(RuntimeError::new(
                "db.query is unavailable for redis connections",
            )),
        }
    }

    fn begin_tx(&mut self) -> Result<(), RuntimeError> {
        match self {
            DbBackend::Sqlite(conn) => conn.exec("BEGIN TRANSACTION").map(|_| ()),
            DbBackend::Postgres(conn) => conn.exec("BEGIN").map(|_| ()),
            DbBackend::Redis(_) => Err(RuntimeError::new(
                "transactions are unavailable for redis connections",
            )),
        }
    }

    fn commit_tx(&mut self) -> Result<(), RuntimeError> {
        match self {
            DbBackend::Sqlite(conn) => conn.exec("COMMIT").map(|_| ()),
            DbBackend::Postgres(conn) => conn.exec("COMMIT").map(|_| ()),
            DbBackend::Redis(_) => Err(RuntimeError::new(
                "transactions are unavailable for redis connections",
            )),
        }
    }

    fn rollback_tx(&mut self) -> Result<(), RuntimeError> {
        match self {
            DbBackend::Sqlite(conn) => conn.exec("ROLLBACK").map(|_| ()),
            DbBackend::Postgres(conn) => conn.exec("ROLLBACK").map(|_| ()),
            DbBackend::Redis(_) => Err(RuntimeError::new(
                "transactions are unavailable for redis connections",
            )),
        }
    }

    fn get(&mut self, key: &str) -> Result<Value, RuntimeError> {
        match self {
            DbBackend::Redis(conn) => conn.get(key),
            _ => Err(RuntimeError::new(
                "db.get is only supported for redis connections",
            )),
        }
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), RuntimeError> {
        match self {
            DbBackend::Redis(conn) => conn.set(key, value),
            _ => Err(RuntimeError::new(
                "db.set is only supported for redis connections",
            )),
        }
    }

    fn del(&mut self, key: &str) -> Result<u64, RuntimeError> {
        match self {
            DbBackend::Redis(conn) => conn.del(key),
            _ => Err(RuntimeError::new(
                "db.del is only supported for redis connections",
            )),
        }
    }
}

impl SqliteConn {
    fn exec(&mut self, sql: &str) -> Result<i64, RuntimeError> {
        self.conn
            .execute(sql, [])
            .map(|rows| rows as i64)
            .map_err(|e| RuntimeError::new(e.to_string()))
    }

    fn query(&mut self, sql: &str) -> Result<Value, RuntimeError> {
        let mut stmt = self
            .conn
            .prepare(sql)
            .map_err(|e| RuntimeError::new(e.to_string()))?;
        let cols = stmt
            .column_names()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let mut rows = stmt
            .query([])
            .map_err(|e| RuntimeError::new(e.to_string()))?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(|e| RuntimeError::new(e.to_string()))? {
            out.push(sqlite_row_to_map(row, &cols));
        }
        Ok(make_vec_value(
            out,
            Some(TypeTag::Map(
                Box::new(TypeTag::Primitive(PrimitiveType::String)),
                Box::new(TypeTag::Unknown),
            )),
        ))
    }
}

impl PostgresConn {
    fn exec(&mut self, sql: &str) -> Result<i64, RuntimeError> {
        self.client
            .execute(sql, &[])
            .map(|rows| rows as i64)
            .map_err(|e| RuntimeError::new(e.to_string()))
    }

    fn query(&mut self, sql: &str) -> Result<Value, RuntimeError> {
        let rows = self
            .client
            .query(sql, &[])
            .map_err(|e| RuntimeError::new(e.to_string()))?;
        let mut out = Vec::new();
        for row in rows {
            out.push(pg_row_to_map(&row));
        }
        Ok(make_vec_value(
            out,
            Some(TypeTag::Map(
                Box::new(TypeTag::Primitive(PrimitiveType::String)),
                Box::new(TypeTag::Unknown),
            )),
        ))
    }
}

impl RedisConn {
    fn get(&mut self, key: &str) -> Result<Value, RuntimeError> {
        match self.conn.get::<_, Option<String>>(key) {
            Ok(Some(s)) => Ok(option_some_value(
                Value::String(s),
                Some(TypeTag::Primitive(PrimitiveType::String)),
            )),
            Ok(None) => Ok(option_none_value(Some(TypeTag::Primitive(
                PrimitiveType::String,
            )))),
            Err(e) => Err(RuntimeError::new(e.to_string())),
        }
    }

    fn set(&mut self, key: &str, value: &str) -> Result<(), RuntimeError> {
        self.conn
            .set::<_, _, ()>(key, value)
            .map_err(|e| RuntimeError::new(e.to_string()))
    }

    fn del(&mut self, key: &str) -> Result<u64, RuntimeError> {
        self.conn
            .del::<_, u64>(key)
            .map_err(|e| RuntimeError::new(e.to_string()))
    }
}

fn pg_row_to_map(row: &PgRow) -> Value {
    let mut fields = HashMap::new();
    for (idx, column) in row.columns().iter().enumerate() {
        let name = column.name().to_string();
        let ty = column.type_().clone();
        let value = pg_value_to_afns(row, idx, ty);
        fields.insert(MapKey::Str(name), value);
    }
    make_map_value(
        fields,
        Some(TypeTag::Primitive(PrimitiveType::String)),
        None,
    )
}

fn pg_value_to_afns(row: &PgRow, idx: usize, ty: PgType) -> Value {
    match ty {
        PgType::BOOL => row
            .try_get::<_, bool>(idx)
            .map(Value::Bool)
            .unwrap_or(Value::Null),
        PgType::INT2 => row
            .try_get::<_, i16>(idx)
            .map(|v| Value::Int(i128::from(v)))
            .unwrap_or(Value::Null),
        PgType::INT4 => row
            .try_get::<_, i32>(idx)
            .map(|v| Value::Int(i128::from(v)))
            .unwrap_or(Value::Null),
        PgType::INT8 => row
            .try_get::<_, i64>(idx)
            .map(|v| Value::Int(v.into()))
            .unwrap_or(Value::Null),
        PgType::FLOAT4 => row
            .try_get::<_, f32>(idx)
            .map(|v| Value::Float(v as f64))
            .unwrap_or(Value::Null),
        PgType::FLOAT8 => row
            .try_get::<_, f64>(idx)
            .map(Value::Float)
            .unwrap_or(Value::Null),
        PgType::TEXT | PgType::VARCHAR | PgType::NAME | PgType::BPCHAR => row
            .try_get::<_, String>(idx)
            .map(Value::String)
            .unwrap_or(Value::Null),
        PgType::BYTEA => row
            .try_get::<_, Vec<u8>>(idx)
            .map(|bytes| {
                make_vec_value(
                    bytes
                        .into_iter()
                        .map(|b| Value::Int(i128::from(b)))
                        .collect(),
                    Some(TypeTag::Primitive(PrimitiveType::Int(IntType::U8))),
                )
            })
            .unwrap_or(Value::Null),
        _ => row
            .try_get::<_, String>(idx)
            .map(Value::String)
            .unwrap_or(Value::Null),
    }
}

fn sqlite_row_to_map(row: &rusqlite::Row<'_>, cols: &[String]) -> Value {
    let mut fields = HashMap::new();
    for (idx, name) in cols.iter().enumerate() {
        let value = match row.get_ref(idx) {
            Ok(ValueRef::Null) => Value::Null,
            Ok(ValueRef::Integer(i)) => Value::Int(i.into()),
            Ok(ValueRef::Real(f)) => Value::Float(f),
            Ok(ValueRef::Text(t)) => Value::String(String::from_utf8_lossy(t).to_string()),
            Ok(ValueRef::Blob(b)) => make_vec_value(
                b.iter().map(|byte| Value::Int(i128::from(*byte))).collect(),
                Some(TypeTag::Primitive(PrimitiveType::Int(IntType::U8))),
            ),
            Err(_) => Value::Null,
        };
        fields.insert(MapKey::Str(name.clone()), value);
    }
    make_map_value(
        fields,
        Some(TypeTag::Primitive(PrimitiveType::String)),
        None,
    )
}
