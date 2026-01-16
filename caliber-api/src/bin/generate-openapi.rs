//! OpenAPI Specification Generator Binary
//!
//! Generates the CALIBER OpenAPI specification as JSON to stdout.
//! Used by SDK generation scripts to create client libraries.
//!
//! Usage:
//!   cargo run -p caliber-api --bin generate-openapi --features openapi > openapi.json

use caliber_api::ApiDoc;
use utoipa::OpenApi;

fn main() {
    let spec = ApiDoc::openapi();

    match serde_json::to_string_pretty(&spec) {
        Ok(json) => println!("{}", json),
        Err(e) => {
            eprintln!("Failed to serialize OpenAPI spec: {}", e);
            std::process::exit(1);
        }
    }
}
