use caliber_dsl::pack::{compose_pack, PackInput, PackMarkdownFile};
use caliber_dsl::parser::parse;
use caliber_dsl::DslCompiler;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::path::PathBuf;

const DSL_MIN: &str = r#"
caliber: "1.0" {
  adapter pg {
    type: postgres
    connection: "postgresql://localhost/caliber"
  }
}
"#;

fn bench_parse_compile(c: &mut Criterion) {
    c.bench_function("dsl/parse_compile_min", |b| {
        b.iter(|| {
            let ast = parse(black_box(DSL_MIN)).expect("parse DSL");
            let compiled = DslCompiler::compile(&ast).expect("compile DSL");
            black_box(compiled.adapters.len());
        });
    });
}

fn bench_pack_compose(c: &mut Criterion) {
    let manifest = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/pack_min/cal.toml"
    ));
    let markdown = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/pack_min/agents/support.md"
    ));

    c.bench_function("pack/compose_min", |b| {
        b.iter(|| {
            let input = PackInput {
                root: PathBuf::from("."),
                manifest: manifest.to_string(),
                markdowns: vec![PackMarkdownFile {
                    path: PathBuf::from("agents/support.md"),
                    content: markdown.to_string(),
                }],
                contracts: std::collections::HashMap::new(),
            };
            let output = compose_pack(input).expect("compose pack");
            black_box(output.compiled.adapters.len());
        });
    });
}

criterion_group!(benches, bench_parse_compile, bench_pack_compose);
criterion_main!(benches);
