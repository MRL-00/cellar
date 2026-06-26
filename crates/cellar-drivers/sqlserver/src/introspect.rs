use std::collections::BTreeMap;

use cellar_core::error::{CellarError, CellarResult};
use cellar_core::schema::{Column, Database, ForeignKey, Index, Schema, Table, View};
use cellar_core::table_browse::{mark_primary_keys, schema_with};

use crate::connect::SqlServerConnection;

pub async fn introspect(conn: &SqlServerConnection) -> CellarResult<Vec<Database>> {
    let schemas = conn
        .with_client(async |client| introspect_schemas(client).await)
        .await?;
    Ok(vec![Database {
        name: conn.config().database.clone(),
        is_default: true,
        schemas,
    }])
}

async fn introspect_schemas(client: &mut crate::connect::TdsClient) -> CellarResult<Vec<Schema>> {
    let schema_names = list_schemas(client).await?;
    let objects = list_objects(client).await?;
    let columns_by_object = list_columns(client).await?;
    let primary_keys = list_primary_keys(client).await?;
    let foreign_keys = list_foreign_keys(client).await?;
    let indexes = list_indexes(client).await?;
    let view_defs = list_view_definitions(client).await?;

    let mut schemas: BTreeMap<String, Schema> = schema_names
        .into_iter()
        .map(|name| {
            (
                name.clone(),
                Schema {
                    name,
                    tables: Vec::new(),
                    views: Vec::new(),
                },
            )
        })
        .collect();

    for (schema_name, object_name, is_view) in objects {
        let key = (schema_name.clone(), object_name.clone());
        let cols = columns_by_object.get(&key).cloned().unwrap_or_default();
        let entry = schemas
            .entry(schema_name.clone())
            .or_insert_with(|| schema_with(schema_name.clone()));
        if is_view {
            entry.views.push(View {
                name: object_name,
                schema: schema_name,
                columns: cols,
                definition: view_defs.get(&key).cloned(),
            });
        } else {
            let pk = primary_keys.get(&key).cloned().unwrap_or_default();
            entry.tables.push(Table {
                name: object_name,
                schema: schema_name,
                row_count: None,
                columns: mark_primary_keys(cols, &pk),
                primary_key: pk,
                foreign_keys: foreign_keys.get(&key).cloned().unwrap_or_default(),
                indexes: indexes.get(&key).cloned().unwrap_or_default(),
            });
        }
    }

    Ok(schemas.into_values().collect())
}

async fn list_schemas(client: &mut crate::connect::TdsClient) -> CellarResult<Vec<String>> {
    let rows = client
        .simple_query(
            "SELECT name FROM sys.schemas \
             WHERE name NOT IN ('sys', 'INFORMATION_SCHEMA') \
             ORDER BY name",
        )
        .await
        .map_err(intro_err)?
        .into_first_result()
        .await
        .map_err(intro_err)?;
    rows.into_iter().map(|r| get_string(&r, "name")).collect()
}

async fn list_objects(
    client: &mut crate::connect::TdsClient,
) -> CellarResult<Vec<(String, String, bool)>> {
    let rows = client
        .simple_query(
            "SELECT s.name AS schema_name, o.name AS object_name, o.type AS object_type \
             FROM sys.objects o \
             JOIN sys.schemas s ON s.schema_id = o.schema_id \
             WHERE o.type IN ('U', 'V') AND o.is_ms_shipped = 0 \
             ORDER BY s.name, o.name",
        )
        .await
        .map_err(intro_err)?
        .into_first_result()
        .await
        .map_err(intro_err)?;
    rows.into_iter()
        .map(|r| {
            let schema = get_string(&r, "schema_name")?;
            let object = get_string(&r, "object_name")?;
            let ty = get_string(&r, "object_type")?;
            // sys.objects.type is CHAR(2), so it comes back space-padded ("V ").
            Ok((schema, object, ty.trim() == "V"))
        })
        .collect()
}

type ColMap = BTreeMap<(String, String), Vec<Column>>;

async fn list_columns(client: &mut crate::connect::TdsClient) -> CellarResult<ColMap> {
    let rows = client
        .simple_query(
            "SELECT s.name AS schema_name, o.name AS object_name, c.name AS column_name, \
                    typ.name AS data_type, CAST(c.max_length AS int) AS max_length, \
                    CAST(c.precision AS int) AS precision, CAST(c.scale AS int) AS scale, \
                    c.is_nullable, dc.definition AS column_default, CAST(c.column_id AS int) AS column_id \
             FROM sys.columns c \
             JOIN sys.objects o ON o.object_id = c.object_id \
             JOIN sys.schemas s ON s.schema_id = o.schema_id \
             JOIN sys.types typ ON typ.user_type_id = c.user_type_id \
             LEFT JOIN sys.default_constraints dc \
                    ON dc.parent_object_id = c.object_id \
                   AND dc.parent_column_id = c.column_id \
             WHERE o.type IN ('U', 'V') AND o.is_ms_shipped = 0 \
             ORDER BY s.name, o.name, c.column_id",
        )
        .await
        .map_err(intro_err)?
        .into_first_result()
        .await
        .map_err(intro_err)?;

    let mut out = ColMap::new();
    for r in rows {
        let schema = get_string(&r, "schema_name")?;
        let object = get_string(&r, "object_name")?;
        let data_type = format_data_type(&r)?;
        let nullable = get_bool(&r, "is_nullable")?;
        let default = r
            .try_get::<&str, _>("column_default")
            .map_err(|e| CellarError::decode(e.to_string()))?
            .map(ToOwned::to_owned);
        let ordinal = get_i32(&r, "column_id")? as u32;
        out.entry((schema, object)).or_default().push(Column {
            name: get_string(&r, "column_name")?,
            data_type,
            nullable,
            default,
            is_primary_key: false,
            ordinal,
            comment: None,
        });
    }
    Ok(out)
}

async fn list_primary_keys(client: &mut crate::connect::TdsClient) -> CellarResult<KeyMap> {
    let rows = client
        .simple_query(
            "SELECT s.name AS schema_name, t.name AS table_name, c.name AS column_name \
             FROM sys.key_constraints kc \
             JOIN sys.tables t ON t.object_id = kc.parent_object_id \
             JOIN sys.schemas s ON s.schema_id = t.schema_id \
             JOIN sys.index_columns ic ON ic.object_id = t.object_id AND ic.index_id = kc.unique_index_id \
             JOIN sys.columns c ON c.object_id = t.object_id AND c.column_id = ic.column_id \
             WHERE kc.type = 'PK' \
             ORDER BY s.name, t.name, ic.key_ordinal",
        )
        .await
        .map_err(intro_err)?
        .into_first_result()
        .await
        .map_err(intro_err)?;
    let mut out = KeyMap::new();
    for r in rows {
        out.entry((
            get_string(&r, "schema_name")?,
            get_string(&r, "table_name")?,
        ))
        .or_default()
        .push(get_string(&r, "column_name")?);
    }
    Ok(out)
}

type KeyMap = BTreeMap<(String, String), Vec<String>>;
type FkMap = BTreeMap<(String, String), Vec<ForeignKey>>;
type IdxMap = BTreeMap<(String, String), Vec<Index>>;

async fn list_foreign_keys(client: &mut crate::connect::TdsClient) -> CellarResult<FkMap> {
    let rows = client
        .simple_query(
            "SELECT fk.name AS fk_name, s.name AS schema_name, t.name AS table_name, \
                    pc.name AS local_column, rs.name AS ref_schema, rt.name AS ref_table, \
                    rc.name AS ref_column, fkc.constraint_column_id \
             FROM sys.foreign_keys fk \
             JOIN sys.foreign_key_columns fkc ON fkc.constraint_object_id = fk.object_id \
             JOIN sys.tables t ON t.object_id = fk.parent_object_id \
             JOIN sys.schemas s ON s.schema_id = t.schema_id \
             JOIN sys.columns pc ON pc.object_id = t.object_id AND pc.column_id = fkc.parent_column_id \
             JOIN sys.tables rt ON rt.object_id = fk.referenced_object_id \
             JOIN sys.schemas rs ON rs.schema_id = rt.schema_id \
             JOIN sys.columns rc ON rc.object_id = rt.object_id AND rc.column_id = fkc.referenced_column_id \
             ORDER BY s.name, t.name, fk.name, fkc.constraint_column_id",
        )
        .await
        .map_err(intro_err)?
        .into_first_result()
        .await
        .map_err(intro_err)?;
    let mut by_constraint: BTreeMap<(String, String, String), ForeignKey> = BTreeMap::new();
    for r in rows {
        let schema = get_string(&r, "schema_name")?;
        let table = get_string(&r, "table_name")?;
        let name = get_string(&r, "fk_name")?;
        let entry = by_constraint
            .entry((schema.clone(), table.clone(), name.clone()))
            .or_insert_with(|| ForeignKey {
                name,
                columns: Vec::new(),
                referenced_schema: get_string(&r, "ref_schema").unwrap_or_default(),
                referenced_table: get_string(&r, "ref_table").unwrap_or_default(),
                referenced_columns: Vec::new(),
            });
        entry.columns.push(get_string(&r, "local_column")?);
        entry.referenced_columns.push(get_string(&r, "ref_column")?);
    }
    let mut out = FkMap::new();
    for ((schema, table, _), fk) in by_constraint {
        out.entry((schema, table)).or_default().push(fk);
    }
    Ok(out)
}

async fn list_indexes(client: &mut crate::connect::TdsClient) -> CellarResult<IdxMap> {
    let rows = client
        .simple_query(
            "SELECT s.name AS schema_name, t.name AS table_name, i.name AS index_name, \
                    i.is_unique, i.is_primary_key, c.name AS column_name, ic.key_ordinal \
             FROM sys.indexes i \
             JOIN sys.tables t ON t.object_id = i.object_id \
             JOIN sys.schemas s ON s.schema_id = t.schema_id \
             JOIN sys.index_columns ic ON ic.object_id = i.object_id AND ic.index_id = i.index_id \
             JOIN sys.columns c ON c.object_id = t.object_id AND c.column_id = ic.column_id \
             WHERE i.name IS NOT NULL AND i.is_hypothetical = 0 AND ic.is_included_column = 0 \
             ORDER BY s.name, t.name, i.name, ic.key_ordinal",
        )
        .await
        .map_err(intro_err)?
        .into_first_result()
        .await
        .map_err(intro_err)?;
    let mut by_index: BTreeMap<(String, String, String), Index> = BTreeMap::new();
    for r in rows {
        let schema = get_string(&r, "schema_name")?;
        let table = get_string(&r, "table_name")?;
        let name = get_string(&r, "index_name")?;
        let entry = by_index
            .entry((schema.clone(), table.clone(), name.clone()))
            .or_insert_with(|| Index {
                name,
                columns: Vec::new(),
                unique: get_bool(&r, "is_unique").unwrap_or(false),
                primary: get_bool(&r, "is_primary_key").unwrap_or(false),
            });
        entry.columns.push(get_string(&r, "column_name")?);
    }
    let mut out = IdxMap::new();
    for ((schema, table, _), index) in by_index {
        out.entry((schema, table)).or_default().push(index);
    }
    Ok(out)
}

async fn list_view_definitions(
    client: &mut crate::connect::TdsClient,
) -> CellarResult<BTreeMap<(String, String), String>> {
    let rows = client
        .simple_query(
            "SELECT s.name AS schema_name, v.name AS view_name, m.definition \
             FROM sys.views v \
             JOIN sys.schemas s ON s.schema_id = v.schema_id \
             LEFT JOIN sys.sql_modules m ON m.object_id = v.object_id",
        )
        .await
        .map_err(intro_err)?
        .into_first_result()
        .await
        .map_err(intro_err)?;
    let mut out = BTreeMap::new();
    for r in rows {
        if let Some(definition) = r
            .try_get::<&str, _>("definition")
            .map_err(|e| CellarError::decode(e.to_string()))?
        {
            out.insert(
                (get_string(&r, "schema_name")?, get_string(&r, "view_name")?),
                definition.to_string(),
            );
        }
    }
    Ok(out)
}

fn format_data_type(row: &tiberius::Row) -> CellarResult<String> {
    let ty = get_string(row, "data_type")?;
    let max_length = get_i32(row, "max_length")?;
    let precision = get_i32(row, "precision")?;
    let scale = get_i32(row, "scale")?;
    let formatted = match ty.as_str() {
        "varchar" | "char" | "binary" | "varbinary" => {
            if max_length == -1 {
                format!("{ty}(max)")
            } else {
                format!("{ty}({max_length})")
            }
        }
        "nvarchar" | "nchar" => {
            if max_length == -1 {
                format!("{ty}(max)")
            } else {
                format!("{ty}({})", max_length / 2)
            }
        }
        "decimal" | "numeric" => format!("{ty}({precision},{scale})"),
        "datetime2" | "datetimeoffset" | "time" => format!("{ty}({scale})"),
        _ => ty,
    };
    Ok(formatted)
}

fn get_string(row: &tiberius::Row, name: &str) -> CellarResult<String> {
    row.try_get::<&str, _>(name)
        .map_err(|e| CellarError::decode(e.to_string()))?
        .map(ToOwned::to_owned)
        .ok_or_else(|| CellarError::decode(format!("SQL Server column {name} was NULL")))
}

fn get_i32(row: &tiberius::Row, name: &str) -> CellarResult<i32> {
    row.try_get::<i32, _>(name)
        .map_err(|e| CellarError::decode(e.to_string()))?
        .ok_or_else(|| CellarError::decode(format!("SQL Server column {name} was NULL")))
}

fn get_bool(row: &tiberius::Row, name: &str) -> CellarResult<bool> {
    row.try_get::<bool, _>(name)
        .map_err(|e| CellarError::decode(e.to_string()))?
        .ok_or_else(|| CellarError::decode(format!("SQL Server column {name} was NULL")))
}

fn intro_err(err: tiberius::error::Error) -> CellarError {
    CellarError::Introspection(err.to_string())
}
