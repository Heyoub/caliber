use caliber_api::db::{DbClient, DbConfig};

pub fn test_db_client() -> DbClient {
    let config = DbConfig::from_env();
    DbClient::from_config(&config).expect("Failed to create database client")
}
