use std::collections::HashMap;

use ext_php_rs::{convert::IntoZval, exception::PhpException, types::Zval};

use crate::{
    utils::{query_params::QueryParameters, result_set::ResultSet, runtime::runtime},
    CONNECTION_REGISTRY,
};

pub fn query(
    conn_id: String,
    stmt: &str,
    parameters: QueryParameters,
) -> Result<Zval, PhpException> {
    let conn_registry = CONNECTION_REGISTRY.lock().unwrap();
    let conn = conn_registry
        .get(&conn_id)
        .ok_or_else(|| PhpException::from("Connection not found"))?;

    let query_result = runtime().block_on(async {
        let mut rows = conn
            .query(stmt, parameters.to_params())
            .await
            .map_err(|e| PhpException::from(e.to_string()))?;
        let mut results: Vec<HashMap<String, libsql::Value>> = Vec::new();
        let mut columns: Vec<String> = Vec::new();

        while let Ok(Some(row)) = rows.next().await {
            for idx in 0..rows.column_count() {
                let column_name = row.column_name(idx as i32).unwrap();
                columns.push(column_name.to_string());
            }
        }

        while let Ok(Some(row)) = rows.next().await {
            let mut result = HashMap::new();
            for idx in 0..rows.column_count() {
                let column_name = row.column_name(idx as i32).unwrap();
                let value = row.get_value(idx).unwrap();
                result.insert(column_name.to_string(), value);
            }
            results.push(result);
        }

        Ok(ResultSet {
            columns,
            rows: results,
            rows_affected: conn.changes(),
            last_insert_rowid: Some(conn.last_insert_rowid()),
        })
    });

    match query_result {
        Ok(result_set) => result_set
            .into_zval(false)
            .map_err(|e| PhpException::from(e.to_string())),
        Err(e) => Err(e),
    }
}
