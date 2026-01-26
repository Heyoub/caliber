use std::sync::Arc;

use caliber_core::ConflictResolution;
use caliber_pcp::{
    AntiSprawlConfig, ContextDagConfig, DosageConfig, GroundingConfig, LintingConfig, PCPConfig,
    PCPRuntime, PruneStrategy, RecoveryConfig, RecoveryFrequency, StalenessConfig,
};

pub fn test_pcp_runtime() -> Arc<PCPRuntime> {
    Arc::new(PCPRuntime::new(make_test_pcp_config()).expect("Failed to create PCP runtime"))
}

fn make_test_pcp_config() -> PCPConfig {
    PCPConfig {
        context_dag: ContextDagConfig {
            max_depth: 10,
            prune_strategy: PruneStrategy::OldestFirst,
        },
        recovery: RecoveryConfig {
            enabled: true,
            frequency: RecoveryFrequency::OnScopeClose,
            max_checkpoints: 5,
        },
        dosage: DosageConfig {
            max_tokens_per_scope: 8000,
            max_artifacts_per_scope: 100,
            max_notes_per_trajectory: 500,
        },
        anti_sprawl: AntiSprawlConfig {
            max_trajectory_depth: 5,
            max_concurrent_scopes: 10,
        },
        grounding: GroundingConfig {
            require_artifact_backing: false,
            contradiction_threshold: 0.85,
            conflict_resolution: ConflictResolution::LastWriteWins,
        },
        linting: LintingConfig {
            max_artifact_size: 1024 * 1024,
            min_confidence_threshold: 0.3,
        },
        staleness: StalenessConfig { stale_hours: 24 * 30 },
    }
}
