use caliber_core::{
    CaliberConfig, ContextAssembler, ContextPackage, ContextPersistence, EntityIdType, RetryConfig,
    ScopeId, SectionPriorities, TrajectoryId, ValidationMode,
};
use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use std::time::Duration;

fn bench_config(token_budget: i32) -> CaliberConfig {
    CaliberConfig {
        token_budget,
        section_priorities: SectionPriorities {
            user: 100,
            system: 90,
            persona: 85,
            artifacts: 80,
            notes: 70,
            history: 60,
            custom: vec![],
        },
        checkpoint_retention: 10,
        stale_threshold: Duration::from_secs(3600),
        contradiction_threshold: 0.8,
        context_window_persistence: ContextPersistence::Ephemeral,
        validation_mode: ValidationMode::OnMutation,
        embedding_provider: None,
        summarization_provider: None,
        llm_retry_config: RetryConfig {
            max_retries: 2,
            initial_backoff: Duration::from_millis(50),
            max_backoff: Duration::from_secs(2),
            backoff_multiplier: 2.0,
        },
        lock_timeout: Duration::from_secs(15),
        message_retention: Duration::from_secs(3600),
        delegation_timeout: Duration::from_secs(120),
    }
}

fn bench_context_assembly(c: &mut Criterion) {
    let config = bench_config(8_000);
    let assembler = ContextAssembler::new(config).expect("build assembler");
    let user_input = include_str!("../src/context.rs");

    c.bench_function("context/assemble_basic", |b| {
        b.iter(|| {
            let pkg = ContextPackage::new(TrajectoryId::now_v7(), ScopeId::now_v7())
                .with_user_input(black_box(user_input.to_string()));
            let window = assembler.assemble(pkg).expect("assemble context");
            black_box(window.used_tokens);
        });
    });
}

criterion_group!(benches, bench_context_assembly);
criterion_main!(benches);
