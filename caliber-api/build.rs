//! Build script for CALIBER API
//!
//! This script compiles the Protocol Buffer definitions into Rust code
//! using tonic-build. The generated code provides gRPC service traits
//! and message types that mirror the REST API.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile the proto file
    tonic_build::configure()
        // Generate server code (we're implementing the service)
        .build_server(true)
        // Generate client code (useful for testing)
        .build_client(true)
        // Use prost types for well-known types
        .compile_well_known_types(true)
        // Compile the caliber.proto file
        .compile_protos(
            &["proto/caliber.proto"],
            &["proto"],
        )?;

    // Tell cargo to rerun this build script if the proto file changes
    println!("cargo:rerun-if-changed=proto/caliber.proto");

    Ok(())
}
