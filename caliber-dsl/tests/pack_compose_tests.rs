use caliber_dsl::pack::{compose_pack, PackInput, PackMarkdownFile};
use std::fs;
use std::path::PathBuf;

#[test]
fn compose_min_pack_to_ast_and_compile() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tests/fixtures/pack_min");
    let manifest = fs::read_to_string(root.join("cal.toml")).expect("read cal.toml");
    let md = fs::read_to_string(root.join("agents/support.md")).expect("read markdown");

    let input = PackInput {
        root: root.clone(),
        manifest,
        markdowns: vec![PackMarkdownFile {
            path: root.join("agents/support.md"),
            content: md,
        }],
    };

    let output = compose_pack(input).expect("compose pack");
    assert_eq!(output.ast.version, "1.0");
    assert!(!output.compiled.adapters.is_empty());
}
