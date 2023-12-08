use async_std::stream::StreamExt;
use sqlx::{Connection, SqliteConnection};
use std::{error::Error, iter};

pub async fn load_db_to_memory(db_uri: &str, mem_uri: &str) -> Result<(), Box<dyn Error>> {
    // Create connections
    let (file_conn, mem_conn) = futures::join!(
        SqliteConnection::connect(db_uri),
        SqliteConnection::connect(&mem_uri)
    );

    let mut file_conn = file_conn?;
    let mut mem_conn = mem_conn?;

    // Read table schema from file database
    let (table_names, schema_sql): (Vec<_>, Vec<_>) = sqlx::query!(
        "SELECT tbl_name as 'tbl_name!', sql as 'sql!' FROM sqlite_master WHERE type='table'"
    )
    .fetch(&mut file_conn)
    .map(|row| {
        let row = row.unwrap();
        (row.tbl_name, row.sql)
    })
    .unzip()
    .await;

    // Run table schema on in-memory database
    let schema_sql: String = iter::once("BEGIN TRANSACTION;".into())
        .chain(schema_sql.into_iter().map(|value| format!("{};", value)))
        .chain(iter::once("END TRANSACTION;".into()))
        .collect();

    sqlx::query(&schema_sql).execute(&mut mem_conn).await?;

    // Read view schema from file database
    let view_sql: Vec<_> = sqlx::query!(
        "SELECT sql as 'sql!' FROM sqlite_master WHERE type='view'"
    )
    .fetch(&mut file_conn)
    .map(|row| {
        let row = row.unwrap();
        row.sql
    })
    .collect()
    .await;

    // Run view schema on in-memory database
    let schema_sql: String = iter::once("BEGIN TRANSACTION;".into())
        .chain(view_sql.into_iter().map(|value| format!("{};", value)))
        .chain(iter::once("END TRANSACTION;".into()))
        .collect();

    sqlx::query(&schema_sql).execute(&mut mem_conn).await?;

    // Attach in-memory database to file connection
    sqlx::query!("ATTACH DATABASE ? as db", mem_uri)
        .execute(&mut file_conn)
        .await?;

    // Copy tables from file database to in-memory database
    let data_sql: String = iter::once("BEGIN TRANSACTION;".into())
        .chain(table_names.into_iter().map(|table_name| {
            format!(
                "INSERT INTO db.{} SELECT * FROM main.{};",
                table_name, table_name
            )
        }))
        .chain(iter::once("END TRANSACTION;".into()))
        .collect();

    sqlx::query(&data_sql).execute(&mut file_conn).await?;

    Ok(())
}
