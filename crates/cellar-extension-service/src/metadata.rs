use crate::service::Request;
use serde_json::{json, Value};
use sqlx::{Postgres, Row, SqliteConnection, Transaction};

fn bound(value: Value) -> Result<Value, String> {
    if serde_json::to_vec(&value)
        .map_err(|_| "Cannot encode metadata")?
        .len()
        > 524288
    {
        Err("Table metadata exceeds response budget".into())
    } else {
        Ok(value)
    }
}

pub async fn sqlite(connection: &mut SqliteConnection, request: &Request) -> Result<Value, String> {
    let table = request.table.as_deref().ok_or("Select a table")?;
    if request.schema.as_deref() != Some("main") {
        return Err("Unknown SQLite schema".into());
    }
    let ddl=sqlx::query_scalar::<_,Option<String>>("SELECT sql FROM sqlite_schema WHERE name=? AND type IN ('table','view') AND name NOT LIKE 'sqlite_%'").bind(table).fetch_optional(&mut *connection).await.map_err(|_|"Table definition unavailable")?.flatten().ok_or("Unknown table")?;
    let indexes=sqlx::query("SELECT name,sql FROM sqlite_schema WHERE type='index' AND tbl_name=? ORDER BY name LIMIT 129").bind(table).fetch_all(&mut *connection).await.map_err(|_|"Index metadata unavailable")?;
    let keys=sqlx::query("SELECT id,seq,\"from\",\"table\",\"to\",on_update,on_delete FROM pragma_foreign_key_list(?) LIMIT 129").bind(table).fetch_all(&mut *connection).await.map_err(|_|"Foreign key metadata unavailable")?;
    let columns = sqlx::query(
        "SELECT name,dflt_value,hidden FROM pragma_table_xinfo(?) ORDER BY cid LIMIT 129",
    )
    .bind(table)
    .fetch_all(&mut *connection)
    .await
    .map_err(|_| "Column defaults unavailable")?;
    bound(
        json!({"ddl":ddl,"indexes":indexes.iter().map(|r|json!({"name":r.get::<String,_>(0),"definition":r.get::<Option<String>,_>(1).unwrap_or("Implicit unique index".into())})).collect::<Vec<_>>(),"foreignKeys":keys.iter().map(|r|json!({"name":format!("fk_{}_{}",r.get::<i64,_>(0),r.get::<i64,_>(1)),"column":r.get::<String,_>(2),"targetSchema":"main","targetTable":r.get::<String,_>(3),"targetColumn":r.get::<Option<String>,_>(4),"definition":format!("ON UPDATE {} ON DELETE {}",r.get::<String,_>(5),r.get::<String,_>(6))})).collect::<Vec<_>>(),"defaults":columns.iter().map(|r|json!({"column":r.get::<String,_>(0),"default":r.get::<Option<String>,_>(1),"generated":r.get::<i64,_>(2)>0})).collect::<Vec<_>>()}),
    )
}

pub async fn postgres(
    transaction: &mut Transaction<'_, Postgres>,
    request: &Request,
) -> Result<Value, String> {
    let schema = request.schema.as_deref().ok_or("Select a schema")?;
    let table = request.table.as_deref().ok_or("Select a table")?;
    let indexes=sqlx::query("SELECT indexname,indexdef FROM pg_indexes WHERE schemaname=$1 AND tablename=$2 ORDER BY indexname LIMIT 129").bind(schema).bind(table).fetch_all(&mut **transaction).await.map_err(|_|"Index metadata unavailable")?;
    let keys=sqlx::query("SELECT con.conname,pg_get_constraintdef(con.oid),tn.nspname,tc.relname FROM pg_constraint con JOIN pg_class c ON c.oid=con.conrelid JOIN pg_namespace n ON n.oid=c.relnamespace JOIN pg_class tc ON tc.oid=con.confrelid JOIN pg_namespace tn ON tn.oid=tc.relnamespace WHERE n.nspname=$1 AND c.relname=$2 AND con.contype='f' ORDER BY con.conname LIMIT 129").bind(schema).bind(table).fetch_all(&mut **transaction).await.map_err(|_|"Foreign key metadata unavailable")?;
    let columns=sqlx::query("SELECT column_name,column_default,is_generated FROM information_schema.columns WHERE table_schema=$1 AND table_name=$2 ORDER BY ordinal_position LIMIT 129").bind(schema).bind(table).fetch_all(&mut **transaction).await.map_err(|_|"Column defaults unavailable")?;
    let view=sqlx::query_scalar::<_,Option<String>>("SELECT view_definition FROM information_schema.views WHERE table_schema=$1 AND table_name=$2").bind(schema).bind(table).fetch_optional(&mut **transaction).await.map_err(|_|"View definition unavailable")?.flatten();
    bound(
        json!({"ddl":view,"indexes":indexes.iter().map(|r|json!({"name":r.get::<String,_>(0),"definition":r.get::<String,_>(1)})).collect::<Vec<_>>(),"foreignKeys":keys.iter().map(|r|json!({"name":r.get::<String,_>(0),"definition":r.get::<String,_>(1),"targetSchema":r.get::<String,_>(2),"targetTable":r.get::<String,_>(3)})).collect::<Vec<_>>(),"defaults":columns.iter().map(|r|json!({"column":r.get::<String,_>(0),"default":r.get::<Option<String>,_>(1),"generated":r.get::<String,_>(2)!="NEVER"})).collect::<Vec<_>>()}),
    )
}
