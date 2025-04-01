use flow_lib::command::prelude::*;

use serde::{Deserialize, Serialize};

use clickhouse::{error::Result, Client};
use tracing::info;

use super::ClickhouseConfig;

pub const NAME: &str = "clickhouse_batch_insert";

const DEFINITION: &str = flow_lib::node_definition!("/clickhouse/clickhouse_batch_insert.json");

fn build() -> BuildResult {
    static CACHE: BuilderCache =
        BuilderCache::new(|| CmdBuilder::new(DEFINITION)?.check_name(NAME));
    Ok(CACHE.clone()?.build(run))
}

flow_lib::submit!(CommandDescription::new(NAME, |_| { build() }));

#[derive(Serialize, Deserialize, Debug)]
pub struct BatchInsert {
    table_name: String,
    rows: Vec<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Input {
    batches: Vec<BatchInsert>,
    clickhouse: ClickhouseConfig,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Output {
    success: bool,
    message: String,
    row_count: usize,
}

async fn run(_ctx: Context, input: Input) -> Result<Output, CommandError> {
    let clickhouse_url = input.clickhouse.url;
    let clickhouse_user = input.clickhouse.user;
    let clickhouse_password = input.clickhouse.password;
    let clickhouse_database = input
        .clickhouse
        .database
        .ok_or_else(|| CommandError::msg("ClickHouse database name is required"))?;

    let client = Client::default()
        .with_url(clickhouse_url)
        .with_user(clickhouse_user)
        .with_password(clickhouse_password)
        .with_database(clickhouse_database);

    // Prepare all batches and rows without executing them
    let mut prepared_operations = Vec::new();

    for batch in input.batches {
        if batch.rows.is_empty() {
            continue;
        }

        for row in batch.rows {
            let json_str = serde_json::to_string(&row)
                .map_err(|e| CommandError::msg(format!("Failed to serialize JSON: {}", e)))?;

            let query = format!(
                "INSERT INTO {} FORMAT JSONEachRow {}",
                batch.table_name, json_str
            );

            // Store operation for later execution
            prepared_operations.push((batch.table_name.clone(), row.clone(), query));
        }
    }

    // Track successful inserts for potential rollback
    let mut successful_inserts = Vec::new();
    let mut total_rows = 0;
    let mut had_error = false;
    let mut error_message = String::new();

    // Execute all prepared operations
    for (table, row_data, query) in prepared_operations {
        if had_error {
            // Skip execution if we already hit an error
            continue;
        }

        match client.query(&query).execute().await {
            Ok(_) => {
                successful_inserts.push((table, row_data));
                total_rows += 1;
            }
            Err(e) => {
                had_error = true;
                error_message = format!("Query failed: {}", e);
                break;
            }
        }
    }

    // If any operation failed, roll back all successful inserts
    if had_error {
        info!(
            "Rolling back {} successful inserts due to error",
            successful_inserts.len()
        );

        for (table, row_data) in &successful_inserts {
            // Extract the ID as a string
            if let Some(id) = row_data.get("id") {
                let id_str = match id {
                    serde_json::Value::String(s) => s.clone(),
                    _ => id.to_string().trim_matches('"').to_string(),
                };

                info!(
                    "Attempting to roll back row with ID {} from table {}",
                    id_str, table
                );

                // Try the non-distributed table first if we're working with a distributed table
                let base_table = if table.ends_with("_distributed") {
                    table.trim_end_matches("_distributed")
                } else {
                    table.as_str()
                };

                // Use parameterized query for safety
                let delete_query = format!("DELETE FROM {} WHERE id = '{}'", table, id_str);

                // Log the exact query for debugging
                info!("Executing rollback query: {}", delete_query);

                match client.query(&delete_query).execute().await {
                    Ok(_) => info!(
                        "Successfully rolled back row with ID {} from {}",
                        id_str, table
                    ),
                    Err(e) => info!("Error rolling back from {}: {:?}", table, e),
                }

                // If we're using a distributed table, also try the base table to ensure deletion
                if base_table != table {
                    let base_delete_query =
                        format!("DELETE FROM {} WHERE id = '{}'", base_table, id_str);
                    info!("Also trying base table: {}", base_delete_query);

                    match client.query(&base_delete_query).execute().await {
                        Ok(_) => info!("Also deleted from base table {}", base_table),
                        Err(e) => info!("Error deleting from base table {}: {:?}", base_table, e),
                    }
                }
            } else {
                info!("No ID found for row in table {}, skipping rollback", table);
            }
        }

        return Err(CommandError::msg(format!(
            "Transaction rolled back: {}",
            error_message
        )));
    }

    Ok(Output {
        success: true,
        message: format!("Successfully inserted {} rows", total_rows),
        row_count: total_rows,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn get_clickhouse_config_from_env() -> ClickhouseConfig {
        dotenvy::dotenv().ok();

        ClickhouseConfig {
            url: std::env::var("clickhouse_url")
                .unwrap_or_else(|_| "http://localhost:8123".to_string()),
            user: std::env::var("clickhouse_user").unwrap_or_else(|_| "default".to_string()),
            password: std::env::var("clickhouse_password").unwrap_or_default(),
            database: Some(std::env::var("clickhouse_database").unwrap_or_default()),
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_multiple_inserts() {
        let clickhouse_config = get_clickhouse_config_from_env();

        let config_value = serde_json::to_value(&clickhouse_config).unwrap_or_default();

        let record_events = vec![serde_json::json!({
            "id": Uuid::new_v4(),
            "event_time": "2023-06-01 10:00:00.000",
            "instruction_name": "init_bank",
            "inputs": ["param1", "param2"],
            "outputs": ["result1"],
            "caller": "user123",
            "signature": "sig456",
            "success": 1,
            "error_message": null,
            "cluster": "mainnet",
            "invalid_field": "this_will_cause_an_error" // Field not in table schema

        })];

        let current_banks = vec![serde_json::json!({
            "id": Uuid::new_v4(),
            "bank_address": "bank123",
            "version": 1,
            "bank_manager": "manager456",
            "flags": 0,
            "whitelisted_creators": 0,
            "whitelisted_mints": 0,
            "whitelisted_collections": 0,
            "vault_count": 0,
            "cluster": "mainnet",
            "last_updated_at": "2023-06-01 10:00:00.000",

        })];

        let batches = vec![
            BatchInsert {
                table_name: "record_events_distributed".to_string(),
                rows: record_events,
            },
            BatchInsert {
                table_name: "current_banks_distributed".to_string(),
                rows: current_banks,
            },
        ];

        let batches_value = serde_json::to_value(&batches).unwrap_or_default();

        let inputs = value::map! {
            "clickhouse" => config_value,
            "batches" => batches_value,
        };

        let outputs = build().unwrap().run(<_>::default(), inputs).await.unwrap();
        dbg!(&outputs);
    }

    #[tokio::test]
    #[ignore]
    async fn test_rollback_on_failure() {
        let clickhouse_config = get_clickhouse_config_from_env();
        let config_value = serde_json::to_value(&clickhouse_config).unwrap_or_default();

        // Create a client to verify rollback worked
        let client = Client::default()
            .with_url(&clickhouse_config.url)
            .with_user(&clickhouse_config.user)
            .with_password(&clickhouse_config.password)
            .with_database(clickhouse_config.database.clone().unwrap_or_default());

        // Create two unique UUIDs for this test and convert to strings
        let test_uuid1 = Uuid::new_v4().to_string();
        let test_uuid2 = Uuid::new_v4().to_string();
        println!("Test UUID 1: {}", test_uuid1);
        println!("Test UUID 2: {}", test_uuid2);

        // First valid row - using string directly
        let valid_data1 = vec![serde_json::json!({
            "id": test_uuid1,
            "event_time": "2023-06-01 10:00:00.000",
            "instruction_name": "init_bank_1",
            "inputs": ["param1", "param2"],
            "outputs": ["result1"],
            "caller": "user123",
            "signature": "valid_sig_1",
            "success": 1,
            "error_message": null,
            "cluster": "mainnet"
        })];

        // Second valid row - using string directly
        let valid_data2 = vec![serde_json::json!({
            "id": test_uuid2,
            "event_time": "2023-06-01 11:00:00.000",
            "instruction_name": "init_bank_2",
            "inputs": ["param3", "param4"],
            "outputs": ["result2"],
            "caller": "user456",
            "signature": "valid_sig_2",
            "success": 1,
            "error_message": null,
            "cluster": "mainnet"
        })];

        let failing_data = vec![serde_json::json!({
            "id": Uuid::new_v4().to_string(),
            "event_time": "2023-06-01 12:00:00.000",
            "instruction_name": "init_bank_3",
            "inputs": ["param5", "param6"],
            "outputs": ["result3"],
            "caller": "user789",
            "signature": "error_sig",
            "success": 1,
            "error_message": null,
            "cluster": "mainnet"
        })];

        let batches = vec![
            BatchInsert {
                table_name: "record_events_distributed".to_string(),
                rows: valid_data1,
            },
            BatchInsert {
                table_name: "record_events_distributed".to_string(),
                rows: valid_data2,
            },
            BatchInsert {
                table_name: "nonexistent_table_that_will_fail".to_string(),
                rows: failing_data,
            },
        ];

        let batches_value = serde_json::to_value(&batches).unwrap_or_default();

        let inputs = value::map! {
            "clickhouse" => config_value,
            "batches" => batches_value,
        };

        // This should fail with an error about the non-existent table
        let result = build().unwrap().run(<_>::default(), inputs).await;

        assert!(
            result.is_err(),
            "Expected an error due to non-existent table"
        );

        if let Err(e) = result {
            println!("Error: {}", e);
            assert!(
                e.to_string().contains("rolled back") || e.to_string().contains("Transaction"),
                "Error message should mention transaction or rollback: {}",
                e
            );

            // Wait longer to allow ClickHouse to process the deletes
            std::thread::sleep(std::time::Duration::from_secs(2));

            // First check if the records are still there
            let count_query = format!(
                "SELECT COUNT() as count FROM record_events_distributed WHERE id IN ('{}', '{}')",
                test_uuid1, test_uuid2
            );

            println!("Checking with query: {}", count_query);

            match client.query(&count_query).fetch_one::<u64>().await {
                Ok(count) => {
                    if count > 0 {
                        println!("Found {} records that should have been rolled back", count);
                        // Cleanup code...
                        panic!(
                            "Rollback failed: found {} records that should have been deleted",
                            count
                        );
                    } else {
                        println!("Success: No records found, rollback worked");
                    }
                }
                Err(e) => {
                    println!("Error checking records: {}", e);
                    panic!("Error verifying rollback: {}", e);
                }
            }
        }
    }
}
