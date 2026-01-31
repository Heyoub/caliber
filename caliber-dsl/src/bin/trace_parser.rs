//! Pack Markdown Tracer - Shows the flow through Markdown → Config → AST
//!
//! Usage: cargo run --bin trace_parser <markdown-file>

use caliber_dsl::pack::{compose_pack, PackInput, PackMarkdownFile};
use caliber_dsl::config::ast_to_markdown;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run --bin trace_parser <markdown-file>");
        eprintln!();
        eprintln!("Example:");
        eprintln!("  cargo run --bin trace_parser prompts/main.md");
        std::process::exit(1);
    }

    let md_path = &args[1];

    println!("╔═══════════════════════════════════════════════════════════════");
    println!("║ PACK MARKDOWN PARSER TRACER");
    println!("╚═══════════════════════════════════════════════════════════════\n");

    // Read markdown file
    let content = match fs::read_to_string(md_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("❌ Failed to read {}: {}", md_path, e);
            std::process::exit(1);
        }
    };

    println!("📝 INPUT MARKDOWN:");
    println!("{}", content);
    println!();

    // Create PackInput (use a minimal test manifest)
    let manifest_toml = r#"
[meta]
name = "test"
version = "1.0"

[tools]
bin = {}
prompts = {}

[profiles]

[agents]

[toolsets]

[adapters]

[providers]

[policies]

[injections]
"#;

    let input = PackInput {
        root: PathBuf::from("."),
        manifest: manifest_toml.to_string(),
        markdowns: vec![PackMarkdownFile {
            path: PathBuf::from(md_path),
            content,
        }],
        contracts: HashMap::new(),
    };

    // Step 1: Parse markdown → AST
    println!("🔍 PACK PARSER OUTPUT:");
    println!("─────────────────────────────────────────────────────────────");
    match compose_pack(input) {
        Ok(output) => {
            println!("AST version: {}", output.ast.version);
            println!("Definitions count: {}", output.ast.definitions.len());
            println!();

            println!("🌳 AST:");
            println!("─────────────────────────────────────────────────────────────");
            println!("{:#?}", output.ast);
            println!();

            // Step 2: Round-trip test
            println!("🔄 ROUND-TRIP TEST:");
            println!("─────────────────────────────────────────────────────────────");
            let canonical = ast_to_markdown(&output.ast);
            println!("Canonical Markdown:");
            println!("{}", canonical);
            println!();

            println!("✅ Parse succeeded!");
        }
        Err(e) => {
            eprintln!("❌ Parse error: {:?}", e);
            std::process::exit(1);
        }
    }
}
