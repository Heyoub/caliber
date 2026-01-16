//! Build script for CALIBER API
//!
//! This script compiles the Protocol Buffer definitions into Rust code
//! using tonic-build. The generated code provides gRPC service traits
//! and message types that mirror the REST API.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_well_known_types(true)
        .compile(
            &["proto/caliber.proto"],
            &["proto"],
        )?;

    println!("cargo:rerun-if-changed=proto/caliber.proto");

    Ok(())
}
