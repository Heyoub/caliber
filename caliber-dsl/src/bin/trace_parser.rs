/// Parser Tracer - Shows the token flow through lexer → parser → AST
///
/// Usage: cargo run --bin trace_parser "adapter oN { type: postgres connection: \"db\" }"

use caliber_dsl::lexer::Lexer;
use caliber_dsl::parser::{parse, pretty_print};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: cargo run --bin trace_parser \"<DSL code>\"");
        eprintln!();
        eprintln!("Example:");
        eprintln!("  cargo run --bin trace_parser 'caliber: \"1.0\" {{ adapter oN {{ type: postgres connection: \"db\" }} }}'");
        std::process::exit(1);
    }

    let source = &args[1..].join(" ");

    println!("╔═══════════════════════════════════════════════════════════════");
    println!("║ DSL PARSER TRACER");
    println!("╚═══════════════════════════════════════════════════════════════\n");

    println!("📝 INPUT DSL:");
    println!("{}", source);
    println!();

    // Step 1: Lexer
    println!("🔍 LEXER OUTPUT (Tokens):");
    println!("─────────────────────────────────────────────────────────────");
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();

    for (i, token) in tokens.iter().enumerate() {
        println!("{:3}: {:?}", i, token);
    }
    println!();

    // Step 2: Parser
    println!("🌳 PARSER OUTPUT (AST):");
    println!("─────────────────────────────────────────────────────────────");
    match parse(source) {
        Ok(ast) => {
            println!("{:#?}", ast);
            println!();

            // Step 3: Round-trip test
            println!("🔄 ROUND-TRIP TEST:");
            println!("─────────────────────────────────────────────────────────────");
            let pretty = pretty_print(&ast);
            println!("Pretty-printed:");
            println!("{}", pretty);
            println!();

            // Check if input == output (lossless round-trip)
            let source_normalized = source.trim();
            let pretty_normalized = pretty.trim();

            if source_normalized == pretty_normalized {
                println!("✅ INPUT-OUTPUT MATCH - Lossless round-trip!");
            } else {
                println!("⚠️  INPUT-OUTPUT DIFFER - Information lost!");
                println!();
                println!("Original input:");
                println!("{}", source);
                println!();
                println!("Pretty-printed:");
                println!("{}", pretty);
            }
            println!();

            println!("Re-parsing pretty-printed output...");
            match parse(&pretty) {
                Ok(ast2) => {
                    if ast == ast2 {
                        println!("✅ AST STABILITY - Re-parsing produces same AST");
                    } else {
                        println!("❌ AST INSTABILITY - Re-parsing changed the AST!");
                        println!();
                        println!("DIFF:");
                        println!("Original: {:#?}", ast);
                        println!();
                        println!("Re-parsed: {:#?}", ast2);
                    }
                }
                Err(e) => {
                    println!("❌ Re-parse failed: {:?}", e);
                }
            }
        }
        Err(e) => {
            println!("❌ Parse error: {:?}", e);
        }
    }
}
