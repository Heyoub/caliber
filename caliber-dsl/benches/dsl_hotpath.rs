use caliber_dsl::pack::{compose_pack, PackInput, PackMarkdownFile};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::path::PathBuf;

// Markdown-based minimal config for benchmarking
const MARKDOWN_MIN: &str = r#"
# System

Test system prompt

## PCP

Test PCP

### User

```adapter pg
adapter_type: postgres
connection: "postgresql://localhost/caliber"
```
"#;

/// Registers a Criterion benchmark that measures composing a pack from a minimal manifest and a single markdown file.
///
/// The benchmark named "markdown/parse_compile_min" constructs a PackInput containing an inlined minimal manifest and one markdown file (MARKDOWN_MIN) and measures the cost of calling `compose_pack`.
///
/// # Examples
///
/// ```
/// use criterion::Criterion;
/// // In a real benchmark harness Criterion is provided by the test runner.
/// let mut c = Criterion::default();
/// bench_parse_compile(&mut c);
/// ```
fn bench_parse_compile(c: &mut Criterion) {
    c.bench_function("markdown/parse_compile_min", |b| {
        let manifest_toml = r#"
[meta]
name = "bench"
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

        b.iter(|| {
            let input = PackInput {
                root: PathBuf::from("."),
                manifest: black_box(manifest_toml.to_string()),
                markdowns: vec![PackMarkdownFile {
                    path: PathBuf::from("test.md"),
                    content: black_box(MARKDOWN_MIN.to_string()),
                }],
                contracts: std::collections::HashMap::new(),
            };
            let output = compose_pack(input).expect("compose pack");
            black_box(output.compiled.adapters.len());
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
