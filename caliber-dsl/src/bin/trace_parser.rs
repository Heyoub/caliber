/// Pack Markdown Tracer - Shows the flow through Markdown → Config → AST
///
/// Usage: cargo run --bin trace_parser <markdown-file>

use caliber_dsl::pack::{compose_pack, PackInput, PackMarkdownFile};
use caliber_dsl::config::ast_to_markdown;
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

    let manifest_path = PathBuf::from("/tmp/test-manifest.toml");
    fs::write(&manifest_path, manifest_toml).expect("Failed to write temp manifest");

    let input = PackInput {
        manifest: manifest_path.clone(),
        markdowns: vec![PackMarkdownFile {
            path: PathBuf::from(md_path),
            content,
        }],
    };

    // Step 1: Parse markdown → PackIr
    println!("🔍 PACK PARSER OUTPUT:");
    println!("─────────────────────────────────────────────────────────────");
    match compose_pack(input) {
        Ok(output) => {
            println!("Pack: {}", output.pack.meta.name);
            println!("Version: {}", output.pack.meta.version.as_ref().unwrap_or(&"1.0".to_string()));
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
            println!("❌ Parse error: {:?}", e);
        }
    }

    // Cleanup
    let _ = fs::remove_file(manifest_path);
}
