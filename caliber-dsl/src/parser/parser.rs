//! Parser implementation

use super::ast::*;
use crate::lexer::*;

impl Parser {
    /// Parse a lifecycle configuration.
    pub(crate) fn parse_lifecycle(&mut self) -> Result<Lifecycle, ParseError> {
        match &self.current().kind {
            TokenKind::Explicit => {
                self.advance();
                Ok(Lifecycle::Explicit)
            }
            TokenKind::Identifier(s) if s == "auto_close" => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let trigger = self.parse_trigger()?;
                self.expect(TokenKind::RParen)?;
                Ok(Lifecycle::AutoClose(trigger))
            }
            _ => Err(self.error("Expected lifecycle (explicit or auto_close)")),
        }
    }

    /// Parse a trigger.
    pub(crate) fn parse_trigger(&mut self) -> Result<Trigger, ParseError> {
        match &self.current().kind {
            TokenKind::TaskStart => {
                self.advance();
                Ok(Trigger::TaskStart)
            }
            TokenKind::TaskEnd => {
                self.advance();
                Ok(Trigger::TaskEnd)
            }
            TokenKind::ScopeClose => {
                self.advance();
                Ok(Trigger::ScopeClose)
            }
            TokenKind::TurnEnd => {
                self.advance();
                Ok(Trigger::TurnEnd)
            }
            TokenKind::Manual => {
                self.advance();
                Ok(Trigger::Manual)
            }
            TokenKind::Schedule => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let cron = self.expect_string()?;
                self.expect(TokenKind::RParen)?;
                Ok(Trigger::Schedule(cron))
            }
            _ => Err(self.error("Expected trigger")),
        }
    }

    /// Parse an index definition.
    pub(crate) fn parse_index_def(&mut self) -> Result<IndexDef, ParseError> {
        let field = self.expect_field_name()?;
        self.expect(TokenKind::Colon)?;
        let index_type = self.parse_index_type()?;
        let mut options = Vec::new();

        if self.check(&TokenKind::Options) {
            self.advance();
            self.expect(TokenKind::Colon)?;
            self.expect(TokenKind::LBrace)?;
            while !self.check(&TokenKind::RBrace) {
                let key = self.expect_string()?;
                self.expect(TokenKind::Colon)?;
                let value = self.expect_string_or_number()?;
                options.push((key, value));
                self.optional_comma();
            }
            self.expect(TokenKind::RBrace)?;
        }

        Ok(IndexDef {
            field,
            index_type,
            options,
        })
    }

    /// Parse an index type.
    pub(crate) fn parse_index_type(&mut self) -> Result<IndexType, ParseError> {
        match &self.current().kind {
            TokenKind::Btree => {
                self.advance();
                Ok(IndexType::Btree)
            }
            TokenKind::Hash => {
                self.advance();
                Ok(IndexType::Hash)
            }
            TokenKind::Gin => {
                self.advance();
                Ok(IndexType::Gin)
            }
            TokenKind::Hnsw => {
                self.advance();
                Ok(IndexType::Hnsw)
            }
            TokenKind::Ivfflat => {
                self.advance();
                Ok(IndexType::Ivfflat)
            }
            _ => Err(self.error("Expected index type")),
        }
    }

    /// Parse a policy definition (Task 4.5).
    pub(crate) fn parse_policy(&mut self) -> Result<PolicyDef, ParseError> {
        self.expect(TokenKind::Policy)?;
        let name = self.expect_identifier()?;
        self.expect(TokenKind::LBrace)?;

        let mut rules = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            if self.check(&TokenKind::On) {
                self.advance();
                let trigger = self.parse_trigger()?;
                self.expect(TokenKind::Colon)?;
                self.expect(TokenKind::LBracket)?;

                let mut actions = Vec::new();
                while !self.check(&TokenKind::RBracket) {
                    actions.push(self.parse_action()?);
                    self.optional_comma();
                }
                self.expect(TokenKind::RBracket)?;

                rules.push(PolicyRule { trigger, actions });
            } else {
                return Err(self.error("Expected 'on' trigger"));
            }
        }

        self.expect(TokenKind::RBrace)?;

        Ok(PolicyDef { name, rules })
    }

    /// Parse an action.
    pub(crate) fn parse_action(&mut self) -> Result<Action, ParseError> {
        match &self.current().kind {
            TokenKind::Summarize => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let target = self.expect_field_name()?;
                self.expect(TokenKind::RParen)?;
                Ok(Action::Summarize(target))
            }
            TokenKind::ExtractArtifacts => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let target = self.expect_field_name()?;
                self.expect(TokenKind::RParen)?;
                Ok(Action::ExtractArtifacts(target))
            }
            TokenKind::Checkpoint => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let target = self.expect_field_name()?;
                self.expect(TokenKind::RParen)?;
                Ok(Action::Checkpoint(target))
            }
            TokenKind::Prune => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let target = self.expect_field_name()?;
                self.expect(TokenKind::Comma)?;
                let criteria = self.parse_filter_expr()?;
                self.expect(TokenKind::RParen)?;
                Ok(Action::Prune { target, criteria })
            }
            TokenKind::Notify => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let channel = self.expect_string()?;
                self.expect(TokenKind::RParen)?;
                Ok(Action::Notify(channel))
            }
            TokenKind::Inject => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let target = self.expect_field_name()?;
                self.expect(TokenKind::Comma)?;
                let mode = self.parse_injection_mode()?;
                self.expect(TokenKind::RParen)?;
                Ok(Action::Inject { target, mode })
            }
            // Battle Intel Feature 4: Auto-summarize action
            // Syntax: auto_summarize(raw, summary, create_edges: true)
            TokenKind::AutoSummarize => {
                self.advance();
                self.expect(TokenKind::LParen)?;

                // Parse source_level
                let source_level = self.parse_abstraction_level()?;
                self.expect(TokenKind::Comma)?;

                // Parse target_level
                let target_level = self.parse_abstraction_level()?;
                self.expect(TokenKind::Comma)?;

                // Parse create_edges: bool (named parameter)
                let field = self.expect_field_name()?;
                if field != "create_edges" {
                    return Err(self.error("Expected 'create_edges:' parameter"));
                }
                self.expect(TokenKind::Colon)?;
                let create_edges = self.parse_bool()?;

                self.expect(TokenKind::RParen)?;

                Ok(Action::AutoSummarize {
                    source_level,
                    target_level,
                    create_edges,
                })
            }
            _ => Err(self.error("Expected action")),
        }
    }

    /// Parse an injection definition (Task 4.6).
    /// Requires: priority (no defaults per REQ-5)
    pub(crate) fn parse_injection(&mut self) -> Result<InjectionDef, ParseError> {
        self.expect(TokenKind::Inject)?;
        let source = self.expect_field_name()?;
        self.expect(TokenKind::Into)?;
        let target = self.expect_field_name()?;
        self.expect(TokenKind::LBrace)?;

        let mut mode = InjectionMode::Full;
        let mut priority: Option<i32> = None;
        let mut max_tokens = None;
        let mut filter = None;

        while !self.check(&TokenKind::RBrace) {
            let field = self.expect_field_name()?;
            self.expect(TokenKind::Colon)?;

            match field.as_str() {
                "mode" => mode = self.parse_injection_mode()?,
                "priority" => priority = Some(self.expect_number()? as i32),
                "max_tokens" => max_tokens = Some(self.expect_number()? as i32),
                "filter" => filter = Some(self.parse_filter_expr()?),
                _ => return Err(self.error(&format!("unknown field: {}", field))),
            }
        }

        self.expect(TokenKind::RBrace)?;

        // Validate required fields - no defaults allowed
        let priority = priority.ok_or_else(|| self.error("missing required field: priority"))?;

        Ok(InjectionDef {
            source,
            target,
            mode,
            priority,
            max_tokens,
            filter,
        })
    }

    /// Parse an injection mode.
    pub(crate) fn parse_injection_mode(&mut self) -> Result<InjectionMode, ParseError> {
        match &self.current().kind {
            TokenKind::Full => {
                self.advance();
                Ok(InjectionMode::Full)
            }
            TokenKind::Summary => {
                self.advance();
                Ok(InjectionMode::Summary)
            }
            TokenKind::TopK => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let k = self.expect_number()? as usize;
                self.expect(TokenKind::RParen)?;
                Ok(InjectionMode::TopK(k))
            }
            TokenKind::Relevant => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let threshold = self.expect_number()? as f32;
                self.expect(TokenKind::RParen)?;
                Ok(InjectionMode::Relevant(threshold))
            }
            _ => Err(self.error("Expected injection mode")),
        }
    }

    // ========================================================================
    // BATTLE INTEL FEATURE 3: Evolution Mode Parser
    // ========================================================================

    /// Parse an evolution definition.
    ///
    /// Syntax:
    /// ```text
    /// evolve "config_name" {
    ///     baseline: "snapshot_name"
    ///     candidates: ["config1", "config2"]
    ///     benchmark_queries: 1000
    ///     metrics: ["latency", "throughput"]
    /// }
    /// ```
    pub(crate) fn parse_evolution(&mut self) -> Result<EvolutionDef, ParseError> {
        self.expect(TokenKind::Evolve)?;

        // Parse the evolution name (string literal)
        let name = self.expect_string()?;

        self.expect(TokenKind::LBrace)?;

        let mut baseline: Option<String> = None;
        let mut candidates: Vec<String> = Vec::new();
        let mut benchmark_queries: Option<i32> = None;
        let mut metrics: Vec<String> = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            let field = self.expect_field_name()?;
            self.expect(TokenKind::Colon)?;

            match field.as_str() {
                "baseline" => {
                    baseline = Some(self.expect_string()?);
                }
                "candidates" => {
                    self.expect(TokenKind::LBracket)?;
                    while !self.check(&TokenKind::RBracket) {
                        candidates.push(self.expect_string()?);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBracket)?;
                }
                "benchmark_queries" => {
                    benchmark_queries = Some(self.expect_number()? as i32);
                }
                "metrics" => {
                    self.expect(TokenKind::LBracket)?;
                    while !self.check(&TokenKind::RBracket) {
                        metrics.push(self.expect_string()?);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBracket)?;
                }
                _ => return Err(self.error(&format!("unknown evolution field: {}", field))),
            }
        }

        self.expect(TokenKind::RBrace)?;

        // Validate required fields (no defaults per REQ-5)
        let baseline = baseline.ok_or_else(|| self.error("missing required field: baseline"))?;
        let benchmark_queries = benchmark_queries
            .ok_or_else(|| self.error("missing required field: benchmark_queries"))?;

        if candidates.is_empty() {
            return Err(self.error("candidates must contain at least one config name"));
        }

        Ok(EvolutionDef {
            name,
            baseline,
            candidates,
            benchmark_queries,
            metrics,
        })
    }

    // ========================================================================
    // BATTLE INTEL FEATURE 4: Summarization Policy Parser
    // ========================================================================

    /// Parse a summarization policy definition.
    ///
    /// Syntax:
    /// ```text
    /// summarization_policy "policy_name" {
    ///     triggers: [dosage_reached(80), scope_close, turn_count(5)]
    ///     source_level: raw
    ///     target_level: summary
    ///     max_sources: 20
    ///     create_edges: true
    /// }
    /// ```
    pub(crate) fn parse_summarization_policy(&mut self) -> Result<SummarizationPolicyDef, ParseError> {
        self.expect(TokenKind::SummarizationPolicy)?;

        // Parse the policy name (string literal)
        let name = self.expect_string()?;

        self.expect(TokenKind::LBrace)?;

        let mut triggers: Vec<SummarizationTriggerDsl> = Vec::new();
        let mut source_level: Option<AbstractionLevelDsl> = None;
        let mut target_level: Option<AbstractionLevelDsl> = None;
        let mut max_sources: Option<i32> = None;
        let mut create_edges: Option<bool> = None;

        while !self.check(&TokenKind::RBrace) {
            let field = self.expect_field_name()?;
            self.expect(TokenKind::Colon)?;

            match field.as_str() {
                "triggers" => {
                    self.expect(TokenKind::LBracket)?;
                    while !self.check(&TokenKind::RBracket) {
                        triggers.push(self.parse_summarization_trigger()?);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBracket)?;
                }
                "source_level" => {
                    source_level = Some(self.parse_abstraction_level()?);
                }
                "target_level" => {
                    target_level = Some(self.parse_abstraction_level()?);
                }
                "max_sources" => {
                    max_sources = Some(self.expect_number()? as i32);
                }
                "create_edges" => {
                    create_edges = Some(self.parse_bool()?);
                }
                _ => {
                    return Err(
                        self.error(&format!("unknown summarization_policy field: {}", field))
                    )
                }
            }
        }

        self.expect(TokenKind::RBrace)?;

        // Validate required fields (no defaults per REQ-5)
        let source_level =
            source_level.ok_or_else(|| self.error("missing required field: source_level"))?;
        let target_level =
            target_level.ok_or_else(|| self.error("missing required field: target_level"))?;
        let max_sources =
            max_sources.ok_or_else(|| self.error("missing required field: max_sources"))?;
        let create_edges =
            create_edges.ok_or_else(|| self.error("missing required field: create_edges"))?;

        if triggers.is_empty() {
            return Err(self.error("triggers must contain at least one trigger"));
        }

        Ok(SummarizationPolicyDef {
            name,
            triggers,
            source_level,
            target_level,
            max_sources,
            create_edges,
        })
    }

    /// Parse a summarization trigger.
    ///
    /// Triggers:
    /// - dosage_reached(80)  -> DosageThreshold { percent: 80 }
    /// - scope_close         -> ScopeClose
    /// - turn_count(5)       -> TurnCount { count: 5 }
    /// - artifact_count(10)  -> ArtifactCount { count: 10 }
    /// - manual              -> Manual
    pub(crate) fn parse_summarization_trigger(&mut self) -> Result<SummarizationTriggerDsl, ParseError> {
        match &self.current().kind {
            TokenKind::DosageReached => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let percent = self.expect_number()? as u8;
                if percent > 100 {
                    return Err(self.error("dosage_reached percent must be 0-100"));
                }
                self.expect(TokenKind::RParen)?;
                Ok(SummarizationTriggerDsl::DosageThreshold { percent })
            }
            TokenKind::ScopeClose => {
                self.advance();
                Ok(SummarizationTriggerDsl::ScopeClose)
            }
            TokenKind::TurnCount => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let count = self.expect_number()? as i32;
                if count <= 0 {
                    return Err(self.error("turn_count must be positive"));
                }
                self.expect(TokenKind::RParen)?;
                Ok(SummarizationTriggerDsl::TurnCount { count })
            }
            TokenKind::ArtifactCount => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let count = self.expect_number()? as i32;
                if count <= 0 {
                    return Err(self.error("artifact_count must be positive"));
                }
                self.expect(TokenKind::RParen)?;
                Ok(SummarizationTriggerDsl::ArtifactCount { count })
            }
            TokenKind::Manual => {
                self.advance();
                Ok(SummarizationTriggerDsl::Manual)
            }
            _ => Err(self.error(
                "Expected summarization trigger (dosage_reached, scope_close, turn_count, artifact_count, manual)",
            )),
        }
    }

    /// Parse an abstraction level.
    ///
    /// Levels:
    /// - raw       -> AbstractionLevelDsl::Raw
    /// - summary   -> AbstractionLevelDsl::Summary
    /// - principle -> AbstractionLevelDsl::Principle
    pub(crate) fn parse_abstraction_level(&mut self) -> Result<AbstractionLevelDsl, ParseError> {
        match &self.current().kind {
            TokenKind::Raw => {
                self.advance();
                Ok(AbstractionLevelDsl::Raw)
            }
            TokenKind::Summary => {
                self.advance();
                Ok(AbstractionLevelDsl::Summary)
            }
            TokenKind::Principle => {
                self.advance();
                Ok(AbstractionLevelDsl::Principle)
            }
            _ => Err(self.error("Expected abstraction level (raw, summary, principle)")),
        }
    }

    /// Parse a boolean value (true or false).
    pub(crate) fn parse_bool(&mut self) -> Result<bool, ParseError> {
        match &self.current().kind {
            TokenKind::Identifier(s) if s == "true" => {
                self.advance();
                Ok(true)
            }
            TokenKind::Identifier(s) if s == "false" => {
                self.advance();
                Ok(false)
            }
            _ => Err(self.error("Expected boolean (true or false)")),
        }
    }

    /// Parse a filter expression (Task 4.7).
    pub(crate) fn parse_filter_expr(&mut self) -> Result<FilterExpr, ParseError> {
        self.parse_or_expr()
    }

    pub(crate) fn parse_or_expr(&mut self) -> Result<FilterExpr, ParseError> {
        let mut left = self.parse_and_expr()?;

        while self.check(&TokenKind::Or) {
            self.advance();
            let right = self.parse_and_expr()?;
            left = FilterExpr::Or(vec![left, right]);
        }

        Ok(left)
    }

    pub(crate) fn parse_and_expr(&mut self) -> Result<FilterExpr, ParseError> {
        let mut left = self.parse_comparison()?;

        while self.check(&TokenKind::And) {
            self.advance();
            let right = self.parse_comparison()?;
            left = FilterExpr::And(vec![left, right]);
        }

        Ok(left)
    }

    pub(crate) fn parse_comparison(&mut self) -> Result<FilterExpr, ParseError> {
        if self.check(&TokenKind::Not) {
            self.advance();
            let expr = self.parse_comparison()?;
            return Ok(FilterExpr::Not(Box::new(expr)));
        }

        if self.check(&TokenKind::LParen) {
            self.advance();
            let expr = self.parse_filter_expr()?;
            self.expect(TokenKind::RParen)?;
            return Ok(expr);
        }

        let field = self.expect_field_name()?;
        let op = self.parse_compare_op()?;
        let value = self.parse_filter_value()?;

        Ok(FilterExpr::Comparison { field, op, value })
    }

    pub(crate) fn parse_compare_op(&mut self) -> Result<CompareOp, ParseError> {
        match &self.current().kind {
            TokenKind::Eq => {
                self.advance();
                Ok(CompareOp::Eq)
            }
            TokenKind::Ne => {
                self.advance();
                Ok(CompareOp::Ne)
            }
            TokenKind::Gt => {
                self.advance();
                Ok(CompareOp::Gt)
            }
            TokenKind::Lt => {
                self.advance();
                Ok(CompareOp::Lt)
            }
            TokenKind::Ge => {
                self.advance();
                Ok(CompareOp::Ge)
            }
            TokenKind::Le => {
                self.advance();
                Ok(CompareOp::Le)
            }
            TokenKind::Contains => {
                self.advance();
                Ok(CompareOp::Contains)
            }
            TokenKind::Regex => {
                self.advance();
                Ok(CompareOp::Regex)
            }
            TokenKind::In => {
                self.advance();
                Ok(CompareOp::In)
            }
            _ => Err(self.error("Expected comparison operator")),
        }
    }

    pub(crate) fn parse_filter_value(&mut self) -> Result<FilterValue, ParseError> {
        match &self.current().kind {
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(FilterValue::String(s))
            }
            TokenKind::Number(n) => {
                let n = *n;
                self.advance();
                Ok(FilterValue::Number(n))
            }
            TokenKind::Duration(d) => {
                // Convert duration to a string value for now
                let d = d.clone();
                self.advance();
                Ok(FilterValue::String(d))
            }
            TokenKind::Identifier(s) if s == "true" => {
                self.advance();
                Ok(FilterValue::Bool(true))
            }
            TokenKind::Identifier(s) if s == "false" => {
                self.advance();
                Ok(FilterValue::Bool(false))
            }
            TokenKind::Identifier(s) if s == "null" => {
                self.advance();
                Ok(FilterValue::Null)
            }
            TokenKind::Identifier(s) if s == "current_trajectory" => {
                self.advance();
                Ok(FilterValue::CurrentTrajectory)
            }
            TokenKind::Identifier(s) if s == "current_scope" => {
                self.advance();
                Ok(FilterValue::CurrentScope)
            }
            TokenKind::Identifier(s) if s == "now" => {
                self.advance();
                Ok(FilterValue::Now)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut values = Vec::new();
                while !self.check(&TokenKind::RBracket) {
                    values.push(self.parse_filter_value()?);
                    self.optional_comma();
                }
                self.expect(TokenKind::RBracket)?;
                Ok(FilterValue::Array(values))
            }
            _ => Err(self.error("Expected filter value")),
        }
    }

    // ========================================================================
    // DSL-FIRST ARCHITECTURE: Trajectory Parser
    // ========================================================================

    /// Parse a trajectory definition.
    ///
    /// Syntax:
    /// ```text
    /// trajectory "customer_support" {
    ///     description: "Multi-turn customer support interaction"
    ///     agent_type: "support_agent"
    ///     token_budget: 8000
    ///     memory_refs: [artifacts, notes, scopes]
    /// }
    /// ```
    pub(crate) fn parse_trajectory(&mut self) -> Result<TrajectoryDef, ParseError> {
        self.expect(TokenKind::Trajectory)?;

        let name = self.expect_string()?;
        self.expect(TokenKind::LBrace)?;

        let mut description: Option<String> = None;
        let mut agent_type: Option<String> = None;
        let mut token_budget: Option<i32> = None;
        let mut memory_refs: Vec<String> = Vec::new();
        let mut metadata: Option<serde_json::Value> = None;

        while !self.check(&TokenKind::RBrace) {
            let field = self.expect_field_name()?;
            self.expect(TokenKind::Colon)?;

            match field.as_str() {
                "description" => {
                    description = Some(self.expect_string()?);
                }
                "agent_type" => {
                    agent_type = Some(self.expect_string()?);
                }
                "token_budget" => {
                    token_budget = Some(self.expect_number()? as i32);
                }
                "memory_refs" => {
                    self.expect(TokenKind::LBracket)?;
                    while !self.check(&TokenKind::RBracket) {
                        memory_refs.push(self.expect_field_name()?);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBracket)?;
                }
                "metadata" => {
                    // Parse JSON object inline
                    self.expect(TokenKind::LBrace)?;
                    let mut map = serde_json::Map::new();
                    while !self.check(&TokenKind::RBrace) {
                        let key = self.expect_string()?;
                        self.expect(TokenKind::Colon)?;
                        let value = self.parse_json_value()?;
                        map.insert(key, value);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBrace)?;
                    metadata = Some(serde_json::Value::Object(map));
                }
                _ => return Err(self.error(&format!("unknown trajectory field: {}", field))),
            }
        }

        self.expect(TokenKind::RBrace)?;

        // Validate required fields (no defaults per REQ-5)
        let agent_type = agent_type.ok_or_else(|| self.error("missing required field: agent_type"))?;
        let token_budget = token_budget.ok_or_else(|| self.error("missing required field: token_budget"))?;

        Ok(TrajectoryDef {
            name,
            description,
            agent_type,
            token_budget,
            memory_refs,
            metadata,
        })
    }

    // ========================================================================
    // DSL-FIRST ARCHITECTURE: Agent Parser
    // ========================================================================

    /// Parse an agent definition.
    ///
    /// Syntax:
    /// ```text
    /// agent "support_agent" {
    ///     capabilities: ["classify_issue", "search_kb", "escalate"]
    ///     constraints: {
    ///         max_concurrent: 5
    ///         timeout_ms: 30000
    ///     }
    ///     permissions: {
    ///         read: [artifacts, notes, scopes]
    ///         write: [notes, scopes]
    ///         lock: [scopes]
    ///     }
    /// }
    /// ```
    pub(crate) fn parse_agent(&mut self) -> Result<AgentDef, ParseError> {
        self.expect(TokenKind::Agent)?;

        let name = self.expect_string()?;
        self.expect(TokenKind::LBrace)?;

        let mut capabilities: Vec<String> = Vec::new();
        let mut constraints = AgentConstraints::default();
        let mut permissions = PermissionMatrix::default();

        while !self.check(&TokenKind::RBrace) {
            let field = self.expect_field_name()?;
            self.expect(TokenKind::Colon)?;

            match field.as_str() {
                "capabilities" => {
                    self.expect(TokenKind::LBracket)?;
                    while !self.check(&TokenKind::RBracket) {
                        capabilities.push(self.expect_string()?);
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBracket)?;
                }
                "constraints" => {
                    self.expect(TokenKind::LBrace)?;
                    while !self.check(&TokenKind::RBrace) {
                        let constraint_field = self.expect_field_name()?;
                        self.expect(TokenKind::Colon)?;
                        match constraint_field.as_str() {
                            "max_concurrent" => {
                                constraints.max_concurrent = self.expect_number()? as i32;
                            }
                            "timeout_ms" => {
                                constraints.timeout_ms = self.expect_number()? as i64;
                            }
                            _ => return Err(self.error(&format!("unknown constraint field: {}", constraint_field))),
                        }
                    }
                    self.expect(TokenKind::RBrace)?;
                }
                "permissions" => {
                    self.expect(TokenKind::LBrace)?;
                    while !self.check(&TokenKind::RBrace) {
                        let perm_field = self.expect_field_name()?;
                        self.expect(TokenKind::Colon)?;
                        self.expect(TokenKind::LBracket)?;
                        let mut refs = Vec::new();
                        while !self.check(&TokenKind::RBracket) {
                            refs.push(self.expect_field_name()?);
                            self.optional_comma();
                        }
                        self.expect(TokenKind::RBracket)?;

                        match perm_field.as_str() {
                            "read" => permissions.read = refs,
                            "write" => permissions.write = refs,
                            "lock" => permissions.lock = refs,
                            _ => return Err(self.error(&format!("unknown permission type: {}", perm_field))),
                        }
                    }
                    self.expect(TokenKind::RBrace)?;
                }
                _ => return Err(self.error(&format!("unknown agent field: {}", field))),
            }
        }

        self.expect(TokenKind::RBrace)?;

        Ok(AgentDef {
            name,
            capabilities,
            constraints,
            permissions,
        })
    }

    // ========================================================================
    // DSL-FIRST ARCHITECTURE: Cache Parser
    // ========================================================================

    /// Parse a cache definition.
    ///
    /// Syntax:
    /// ```text
    /// cache {
    ///     backend: lmdb
    ///     path: "/var/caliber/cache"
    ///     size_mb: 1024
    ///     default_freshness: best_effort { max_staleness: 60s }
    /// }
    /// ```
    pub(crate) fn parse_cache(&mut self) -> Result<CacheDef, ParseError> {
        self.expect(TokenKind::Cache)?;
        self.expect(TokenKind::LBrace)?;

        let mut backend: Option<CacheBackendType> = None;
        let mut path: Option<String> = None;
        let mut size_mb: Option<i32> = None;
        let mut default_freshness = FreshnessDef::default();
        let mut max_entries: Option<i32> = None;
        let mut ttl: Option<String> = None;

        while !self.check(&TokenKind::RBrace) {
            let field = self.expect_field_name()?;
            self.expect(TokenKind::Colon)?;

            match field.as_str() {
                "backend" => {
                    backend = Some(self.parse_cache_backend()?);
                }
                "path" => {
                    path = Some(self.expect_string()?);
                }
                "size_mb" => {
                    size_mb = Some(self.expect_number()? as i32);
                }
                "default_freshness" => {
                    default_freshness = self.parse_freshness()?;
                }
                "max_entries" => {
                    max_entries = Some(self.expect_number()? as i32);
                }
                "ttl" => {
                    ttl = Some(self.expect_duration()?);
                }
                _ => return Err(self.error(&format!("unknown cache field: {}", field))),
            }
        }

        self.expect(TokenKind::RBrace)?;

        // Validate required fields (no defaults per REQ-5)
        let backend = backend.ok_or_else(|| self.error("missing required field: backend"))?;
        let size_mb = size_mb.ok_or_else(|| self.error("missing required field: size_mb"))?;

        Ok(CacheDef {
            backend,
            path,
            size_mb,
            default_freshness,
            max_entries,
            ttl,
        })
    }

    /// Parse cache backend type.
    fn parse_cache_backend(&mut self) -> Result<CacheBackendType, ParseError> {
        match &self.current().kind {
            TokenKind::Lmdb => {
                self.advance();
                Ok(CacheBackendType::Lmdb)
            }
            TokenKind::Memory => {
                self.advance();
                Ok(CacheBackendType::Memory)
            }
            _ => Err(self.error("Expected cache backend (lmdb, memory)")),
        }
    }

    /// Parse freshness configuration.
    fn parse_freshness(&mut self) -> Result<FreshnessDef, ParseError> {
        match &self.current().kind {
            TokenKind::BestEffort => {
                self.advance();
                self.expect(TokenKind::LBrace)?;
                let mut max_staleness: Option<String> = None;
                while !self.check(&TokenKind::RBrace) {
                    let field = self.expect_field_name()?;
                    self.expect(TokenKind::Colon)?;
                    match field.as_str() {
                        "max_staleness" => {
                            max_staleness = Some(self.expect_duration()?);
                        }
                        _ => return Err(self.error(&format!("unknown freshness field: {}", field))),
                    }
                }
                self.expect(TokenKind::RBrace)?;
                let max_staleness = max_staleness.ok_or_else(|| self.error("missing max_staleness in best_effort"))?;
                Ok(FreshnessDef::BestEffort { max_staleness })
            }
            TokenKind::Strict => {
                self.advance();
                Ok(FreshnessDef::Strict)
            }
            _ => Err(self.error("Expected freshness (best_effort, strict)")),
        }
    }

    /// Parse duration string (e.g., "60s", "5m").
    fn expect_duration(&mut self) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::Duration(d) => {
                let d = d.clone();
                self.advance();
                Ok(d)
            }
            _ => Err(self.error("Expected duration (e.g., 60s, 5m)")),
        }
    }

    // ========================================================================
    // DSL-FIRST ARCHITECTURE: Provider Parser
    // ========================================================================

    /// Parse a provider definition.
    ///
    /// Syntax:
    /// ```text
    /// provider "openai" {
    ///     type: openai
    ///     api_key: env("OPENAI_API_KEY")
    ///     model: "text-embedding-3-small"
    /// }
    /// ```
    pub(crate) fn parse_provider(&mut self) -> Result<ProviderDef, ParseError> {
        self.expect(TokenKind::Provider)?;

        let name = self.expect_string()?;
        self.expect(TokenKind::LBrace)?;

        let mut provider_type: Option<ProviderType> = None;
        let mut api_key: Option<EnvValue> = None;
        let mut model: Option<String> = None;
        let mut options: Vec<(String, String)> = Vec::new();

        while !self.check(&TokenKind::RBrace) {
            let field = self.expect_field_name()?;
            self.expect(TokenKind::Colon)?;

            match field.as_str() {
                "type" => {
                    provider_type = Some(self.parse_provider_type()?);
                }
                "api_key" => {
                    api_key = Some(self.parse_env_value()?);
                }
                "model" => {
                    model = Some(self.expect_string()?);
                }
                "options" => {
                    self.expect(TokenKind::LBrace)?;
                    while !self.check(&TokenKind::RBrace) {
                        let key = self.expect_string()?;
                        self.expect(TokenKind::Colon)?;
                        let value = self.expect_string_or_number()?;
                        options.push((key, value));
                        self.optional_comma();
                    }
                    self.expect(TokenKind::RBrace)?;
                }
                _ => return Err(self.error(&format!("unknown provider field: {}", field))),
            }
        }

        self.expect(TokenKind::RBrace)?;

        // Validate required fields (no defaults per REQ-5)
        let provider_type = provider_type.ok_or_else(|| self.error("missing required field: type"))?;
        let api_key = api_key.ok_or_else(|| self.error("missing required field: api_key"))?;
        let model = model.ok_or_else(|| self.error("missing required field: model"))?;

        Ok(ProviderDef {
            name,
            provider_type,
            api_key,
            model,
            options,
        })
    }

    /// Parse provider type.
    fn parse_provider_type(&mut self) -> Result<ProviderType, ParseError> {
        match &self.current().kind {
            TokenKind::Openai => {
                self.advance();
                Ok(ProviderType::OpenAI)
            }
            TokenKind::Anthropic => {
                self.advance();
                Ok(ProviderType::Anthropic)
            }
            TokenKind::Identifier(s) if s == "custom" => {
                self.advance();
                Ok(ProviderType::Custom)
            }
            _ => Err(self.error("Expected provider type (openai, anthropic, custom)")),
        }
    }

    /// Parse env value (env("VAR") or literal string).
    fn parse_env_value(&mut self) -> Result<EnvValue, ParseError> {
        match &self.current().kind {
            TokenKind::Env => {
                self.advance();
                self.expect(TokenKind::LParen)?;
                let var_name = self.expect_string()?;
                self.expect(TokenKind::RParen)?;
                Ok(EnvValue::Env(var_name))
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(EnvValue::Literal(s))
            }
            _ => Err(self.error("Expected env(\"VAR\") or string literal")),
        }
    }

    /// Parse a JSON value for metadata.
    fn parse_json_value(&mut self) -> Result<serde_json::Value, ParseError> {
        match &self.current().kind {
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(serde_json::Value::String(s))
            }
            TokenKind::Number(n) => {
                let n = *n;
                self.advance();
                Ok(serde_json::json!(n))
            }
            TokenKind::Identifier(s) if s == "true" => {
                self.advance();
                Ok(serde_json::Value::Bool(true))
            }
            TokenKind::Identifier(s) if s == "false" => {
                self.advance();
                Ok(serde_json::Value::Bool(false))
            }
            TokenKind::Identifier(s) if s == "null" => {
                self.advance();
                Ok(serde_json::Value::Null)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut arr = Vec::new();
                while !self.check(&TokenKind::RBracket) {
                    arr.push(self.parse_json_value()?);
                    self.optional_comma();
                }
                self.expect(TokenKind::RBracket)?;
                Ok(serde_json::Value::Array(arr))
            }
            TokenKind::LBrace => {
                self.advance();
                let mut map = serde_json::Map::new();
                while !self.check(&TokenKind::RBrace) {
                    let key = self.expect_string()?;
                    self.expect(TokenKind::Colon)?;
                    let value = self.parse_json_value()?;
                    map.insert(key, value);
                    self.optional_comma();
                }
                self.expect(TokenKind::RBrace)?;
                Ok(serde_json::Value::Object(map))
            }
            _ => Err(self.error("Expected JSON value")),
        }
    }

    // ========================================================================
    // DSL-FIRST ARCHITECTURE: Modifier Parser
    // ========================================================================

    /// Parse a modifier definition.
    ///
    /// Syntax:
    /// ```text
    /// embeddable { provider: "openai" }
    /// summarizable { style: brief, on: [scope_close] }
    /// lockable { mode: exclusive }
    /// ```
    pub(crate) fn parse_modifier(&mut self) -> Result<ModifierDef, ParseError> {
        match &self.current().kind {
            TokenKind::Embeddable => {
                self.advance();
                self.expect(TokenKind::LBrace)?;
                let mut provider: Option<String> = None;
                while !self.check(&TokenKind::RBrace) {
                    let field = self.expect_field_name()?;
                    self.expect(TokenKind::Colon)?;
                    match field.as_str() {
                        "provider" => {
                            provider = Some(self.expect_string()?);
                        }
                        _ => return Err(self.error(&format!("unknown embeddable field: {}", field))),
                    }
                }
                self.expect(TokenKind::RBrace)?;
                let provider = provider.ok_or_else(|| self.error("missing provider in embeddable"))?;
                Ok(ModifierDef::Embeddable { provider })
            }
            TokenKind::Summarizable => {
                self.advance();
                self.expect(TokenKind::LBrace)?;
                let mut style: Option<SummaryStyle> = None;
                let mut on_triggers: Vec<Trigger> = Vec::new();
                while !self.check(&TokenKind::RBrace) {
                    let field = self.expect_field_name()?;
                    self.expect(TokenKind::Colon)?;
                    match field.as_str() {
                        "style" => {
                            style = Some(self.parse_summary_style()?);
                        }
                        "on" => {
                            self.expect(TokenKind::LBracket)?;
                            while !self.check(&TokenKind::RBracket) {
                                on_triggers.push(self.parse_trigger()?);
                                self.optional_comma();
                            }
                            self.expect(TokenKind::RBracket)?;
                        }
                        _ => return Err(self.error(&format!("unknown summarizable field: {}", field))),
                    }
                }
                self.expect(TokenKind::RBrace)?;
                let style = style.ok_or_else(|| self.error("missing style in summarizable"))?;
                Ok(ModifierDef::Summarizable { style, on_triggers })
            }
            TokenKind::Lockable => {
                self.advance();
                self.expect(TokenKind::LBrace)?;
                let mut mode: Option<LockMode> = None;
                while !self.check(&TokenKind::RBrace) {
                    let field = self.expect_field_name()?;
                    self.expect(TokenKind::Colon)?;
                    match field.as_str() {
                        "mode" => {
                            mode = Some(self.parse_lock_mode()?);
                        }
                        _ => return Err(self.error(&format!("unknown lockable field: {}", field))),
                    }
                }
                self.expect(TokenKind::RBrace)?;
                let mode = mode.ok_or_else(|| self.error("missing mode in lockable"))?;
                Ok(ModifierDef::Lockable { mode })
            }
            _ => Err(self.error("Expected modifier (embeddable, summarizable, lockable)")),
        }
    }

    /// Parse summary style.
    fn parse_summary_style(&mut self) -> Result<SummaryStyle, ParseError> {
        match &self.current().kind {
            TokenKind::Brief => {
                self.advance();
                Ok(SummaryStyle::Brief)
            }
            TokenKind::Detailed => {
                self.advance();
                Ok(SummaryStyle::Detailed)
            }
            _ => Err(self.error("Expected summary style (brief, detailed)")),
        }
    }

    /// Parse lock mode.
    fn parse_lock_mode(&mut self) -> Result<LockMode, ParseError> {
        match &self.current().kind {
            TokenKind::Exclusive => {
                self.advance();
                Ok(LockMode::Exclusive)
            }
            TokenKind::Shared => {
                self.advance();
                Ok(LockMode::Shared)
            }
            _ => Err(self.error("Expected lock mode (exclusive, shared)")),
        }
    }

    // ========================================================================
    // Helper methods
    // ========================================================================

    pub(crate) fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    pub(crate) fn advance(&mut self) {
        if !self.is_at_end() {
            self.pos += 1;
        }
    }

    pub(crate) fn is_at_end(&self) -> bool {
        self.current().kind == TokenKind::Eof
    }

    pub(crate) fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(kind)
    }

    pub(crate) fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        if self.check(&kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(&format!("Expected {:?}", kind)))
        }
    }

    pub(crate) fn expect_identifier(&mut self) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::Identifier(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(self.error("Expected identifier")),
        }
    }

    /// Expect an identifier or a keyword that can be used as a field name.
    /// Many keywords in the DSL can also be used as field names (type, mode, filter, etc.)
    pub(crate) fn expect_field_name(&mut self) -> Result<String, ParseError> {
        let name = match &self.current().kind {
            TokenKind::Identifier(s) => s.clone(),
            // Keywords that can be used as field names
            TokenKind::Type => "type".to_string(),
            TokenKind::Mode => "mode".to_string(),
            TokenKind::Filter => "filter".to_string(),
            TokenKind::Schema => "schema".to_string(),
            TokenKind::Retention => "retention".to_string(),
            TokenKind::Index => "index".to_string(),
            TokenKind::Lifecycle => "lifecycle".to_string(),
            TokenKind::Parent => "parent".to_string(),
            TokenKind::InjectOn => "inject_on".to_string(),
            TokenKind::Connection => "connection".to_string(),
            TokenKind::Options => "options".to_string(),
            TokenKind::Priority => "priority".to_string(),
            TokenKind::MaxTokens => "max_tokens".to_string(),
            TokenKind::Schedule => "schedule".to_string(),
            TokenKind::Artifacts => "artifacts".to_string(),
            // Field types that can also be field names
            TokenKind::Embedding => "embedding".to_string(),
            TokenKind::Uuid => "uuid".to_string(),
            TokenKind::Text => "text".to_string(),
            TokenKind::Int => "int".to_string(),
            TokenKind::Float => "float".to_string(),
            TokenKind::Bool => "bool".to_string(),
            TokenKind::Timestamp => "timestamp".to_string(),
            TokenKind::Json => "json".to_string(),
            TokenKind::Enum => "enum".to_string(),
            // Memory types that can be field names
            TokenKind::Ephemeral => "ephemeral".to_string(),
            TokenKind::Working => "working".to_string(),
            TokenKind::Episodic => "episodic".to_string(),
            TokenKind::Semantic => "semantic".to_string(),
            TokenKind::Procedural => "procedural".to_string(),
            TokenKind::Meta => "meta".to_string(),
            TokenKind::Memory => "memory".to_string(),
            // Retention/scope keywords
            TokenKind::Scope => "scope".to_string(),
            TokenKind::Session => "session".to_string(),
            TokenKind::Persistent => "persistent".to_string(),
            // Other keywords that might be field names
            TokenKind::Context => "context".to_string(),
            TokenKind::Inject => "inject".to_string(),
            TokenKind::Policy => "policy".to_string(),
            TokenKind::Adapter => "adapter".to_string(),
            TokenKind::Into => "into".to_string(),
            TokenKind::On => "on".to_string(),
            TokenKind::Caliber => "caliber".to_string(),
            // Lifecycle keywords
            TokenKind::Explicit => "explicit".to_string(),
            TokenKind::Manual => "manual".to_string(),
            TokenKind::TaskStart => "task_start".to_string(),
            TokenKind::TaskEnd => "task_end".to_string(),
            TokenKind::ScopeClose => "scope_close".to_string(),
            TokenKind::TurnEnd => "turn_end".to_string(),
            // Action keywords
            TokenKind::Summarize => "summarize".to_string(),
            TokenKind::ExtractArtifacts => "extract_artifacts".to_string(),
            TokenKind::Checkpoint => "checkpoint".to_string(),
            TokenKind::Prune => "prune".to_string(),
            TokenKind::Notify => "notify".to_string(),
            // Index types
            TokenKind::Btree => "btree".to_string(),
            TokenKind::Hash => "hash".to_string(),
            TokenKind::Gin => "gin".to_string(),
            TokenKind::Hnsw => "hnsw".to_string(),
            TokenKind::Ivfflat => "ivfflat".to_string(),
            // Injection modes
            TokenKind::Full => "full".to_string(),
            TokenKind::Summary => "summary".to_string(),
            TokenKind::TopK => "top_k".to_string(),
            TokenKind::Relevant => "relevant".to_string(),
            // Battle Intel Feature 3 & 4: Evolution and summarization fields
            TokenKind::Evolve => "evolve".to_string(),
            TokenKind::Baseline => "baseline".to_string(),
            TokenKind::Candidates => "candidates".to_string(),
            TokenKind::Metrics => "metrics".to_string(),
            TokenKind::BenchmarkQueries => "benchmark_queries".to_string(),
            TokenKind::Triggers => "triggers".to_string(),
            TokenKind::SourceLevel => "source_level".to_string(),
            TokenKind::TargetLevel => "target_level".to_string(),
            TokenKind::MaxSources => "max_sources".to_string(),
            TokenKind::CreateEdges => "create_edges".to_string(),
            TokenKind::Raw => "raw".to_string(),
            TokenKind::Principle => "principle".to_string(),
            TokenKind::AutoSummarize => "auto_summarize".to_string(),
            TokenKind::DosageReached => "dosage_reached".to_string(),
            TokenKind::TurnCount => "turn_count".to_string(),
            TokenKind::ArtifactCount => "artifact_count".to_string(),
            TokenKind::SummarizationPolicy => "summarization_policy".to_string(),
            TokenKind::Freeze => "freeze".to_string(),
            TokenKind::Snapshot => "snapshot".to_string(),
            TokenKind::Benchmark => "benchmark".to_string(),
            TokenKind::Compare => "compare".to_string(),
            TokenKind::AbstractionLevel => "abstraction_level".to_string(),
            // DSL-first architecture: New keywords as field names
            TokenKind::Trajectory => "trajectory".to_string(),
            TokenKind::Agent => "agent".to_string(),
            TokenKind::Cache => "cache".to_string(),
            TokenKind::Provider => "provider".to_string(),
            TokenKind::Capabilities => "capabilities".to_string(),
            TokenKind::Constraints => "constraints".to_string(),
            TokenKind::Permissions => "permissions".to_string(),
            TokenKind::MaxConcurrent => "max_concurrent".to_string(),
            TokenKind::TimeoutMs => "timeout_ms".to_string(),
            TokenKind::Read => "read".to_string(),
            TokenKind::Write => "write".to_string(),
            TokenKind::Lock => "lock".to_string(),
            TokenKind::Backend => "backend".to_string(),
            TokenKind::Lmdb => "lmdb".to_string(),
            TokenKind::MaxStaleness => "max_staleness".to_string(),
            TokenKind::PollInterval => "poll_interval".to_string(),
            TokenKind::Prefetch => "prefetch".to_string(),
            TokenKind::MaxEntries => "max_entries".to_string(),
            TokenKind::Ttl => "ttl".to_string(),
            TokenKind::SizeMb => "size_mb".to_string(),
            TokenKind::DefaultFreshness => "default_freshness".to_string(),
            TokenKind::BestEffort => "best_effort".to_string(),
            TokenKind::Strict => "strict".to_string(),
            TokenKind::Modifiers => "modifiers".to_string(),
            TokenKind::Embeddable => "embeddable".to_string(),
            TokenKind::Summarizable => "summarizable".to_string(),
            TokenKind::Lockable => "lockable".to_string(),
            TokenKind::Style => "style".to_string(),
            TokenKind::Brief => "brief".to_string(),
            TokenKind::Detailed => "detailed".to_string(),
            TokenKind::Exclusive => "exclusive".to_string(),
            TokenKind::Shared => "shared".to_string(),
            TokenKind::ApiKey => "api_key".to_string(),
            TokenKind::Model => "model".to_string(),
            TokenKind::Openai => "openai".to_string(),
            TokenKind::Anthropic => "anthropic".to_string(),
            TokenKind::Env => "env".to_string(),
            TokenKind::Description => "description".to_string(),
            TokenKind::AgentType => "agent_type".to_string(),
            TokenKind::TokenBudget => "token_budget".to_string(),
            TokenKind::MemoryRefs => "memory_refs".to_string(),
            _ => return Err(self.error("Expected identifier")),
        };
        self.advance();
        Ok(name)
    }

    pub(crate) fn expect_string(&mut self) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(self.error("Expected string")),
        }
    }

    pub(crate) fn expect_number(&mut self) -> Result<f64, ParseError> {
        match self.current().kind {
            TokenKind::Number(n) => {
                self.advance();
                Ok(n)
            }
            _ => Err(self.error("Expected number")),
        }
    }

    pub(crate) fn expect_string_or_number(&mut self) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            TokenKind::Number(n) => {
                let s = n.to_string();
                self.advance();
                Ok(s)
            }
            _ => Err(self.error("Expected string or number")),
        }
    }

    pub(crate) fn optional_comma(&mut self) {
        if self.check(&TokenKind::Comma) {
            self.advance();
        }
    }

    pub(crate) fn error(&self, msg: &str) -> ParseError {
        let span = self.current().span;
        ParseError {
            message: msg.to_string(),
            line: span.line,
            column: span.column,
        }
    }
}


// ============================================================================
// PRETTY PRINTER (Task 4.9)
// ============================================================================

/// Pretty-print a CaliberAst back to DSL source code.
pub fn pretty_print(ast: &CaliberAst) -> String {
    let mut output = String::new();
    output.push_str(&format!("caliber: \"{}\" {{\n", ast.version));

    for def in &ast.definitions {
        output.push_str(&pretty_print_definition(def, 1));
    }

    output.push_str("}\n");
    output
}

fn pretty_print_definition(def: &Definition, indent: usize) -> String {
    match def {
        Definition::Adapter(a) => pretty_print_adapter(a, indent),
        Definition::Memory(m) => pretty_print_memory(m, indent),
        Definition::Policy(p) => pretty_print_policy(p, indent),
        Definition::Injection(i) => pretty_print_injection(i, indent),
        // Battle Intel Feature 3: Evolution definitions
        Definition::Evolution(e) => pretty_print_evolution(e, indent),
        // Battle Intel Feature 4: Summarization policy definitions
        Definition::SummarizationPolicy(s) => pretty_print_summarization_policy(s, indent),
        // DSL-first architecture: New definitions
        Definition::Trajectory(t) => pretty_print_trajectory(t, indent),
        Definition::Agent(a) => pretty_print_agent(a, indent),
        Definition::Cache(c) => pretty_print_cache(c, indent),
        Definition::Provider(p) => pretty_print_provider(p, indent),
    }
}

/// Pretty print an evolution definition (Battle Intel Feature 3).
fn pretty_print_evolution(e: &EvolutionDef, indent: usize) -> String {
    let ind = indent_str(indent);
    let inner_ind = indent_str(indent + 1);
    let mut result = format!("{}evolution \"{}\" {{\n", ind, e.name);
    result.push_str(&format!("{}baseline: \"{}\"\n", inner_ind, e.baseline));
    result.push_str(&format!("{}candidates: [{}]\n", inner_ind,
        e.candidates.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", ")));
    result.push_str(&format!("{}benchmark_queries: {}\n", inner_ind, e.benchmark_queries));
    result.push_str(&format!("{}metrics: [{}]\n", inner_ind,
        e.metrics.iter().map(|m| format!("\"{}\"", m)).collect::<Vec<_>>().join(", ")));
    result.push_str(&format!("{}}}\n", ind));
    result
}

/// Pretty print a summarization policy definition (Battle Intel Feature 4).
fn pretty_print_summarization_policy(s: &SummarizationPolicyDef, indent: usize) -> String {
    let ind = indent_str(indent);
    let inner_ind = indent_str(indent + 1);
    let mut result = format!("{}summarization_policy \"{}\" {{\n", ind, s.name);
    result.push_str(&format!("{}triggers: [{}]\n", inner_ind,
        s.triggers.iter().map(pretty_print_summarization_trigger).collect::<Vec<_>>().join(", ")));
    result.push_str(&format!("{}source_level: {}\n", inner_ind, pretty_print_abstraction_level(s.source_level)));
    result.push_str(&format!("{}target_level: {}\n", inner_ind, pretty_print_abstraction_level(s.target_level)));
    result.push_str(&format!("{}max_sources: {}\n", inner_ind, s.max_sources));
    result.push_str(&format!("{}create_edges: {}\n", inner_ind, s.create_edges));
    result.push_str(&format!("{}}}\n", ind));
    result
}

fn pretty_print_summarization_trigger(t: &SummarizationTriggerDsl) -> String {
    match t {
        SummarizationTriggerDsl::DosageThreshold { percent } => format!("dosage_reached({})", percent),
        SummarizationTriggerDsl::ScopeClose => "scope_close".to_string(),
        SummarizationTriggerDsl::TurnCount { count } => format!("turn_count({})", count),
        SummarizationTriggerDsl::ArtifactCount { count } => format!("artifact_count({})", count),
        SummarizationTriggerDsl::Manual => "manual".to_string(),
    }
}

fn pretty_print_abstraction_level(level: AbstractionLevelDsl) -> &'static str {
    match level {
        AbstractionLevelDsl::Raw => "raw",
        AbstractionLevelDsl::Summary => "summary",
        AbstractionLevelDsl::Principle => "principle",
    }
}

fn indent_str(level: usize) -> String {
    "    ".repeat(level)
}

fn pretty_print_adapter(adapter: &AdapterDef, indent: usize) -> String {
    let mut output = String::new();
    let ind = indent_str(indent);

    output.push_str(&format!("{}adapter {} {{\n", ind, adapter.name));
    output.push_str(&format!("{}type: {}\n", indent_str(indent + 1), pretty_print_adapter_type(&adapter.adapter_type)));
    output.push_str(&format!("{}connection: \"{}\"\n", indent_str(indent + 1), escape_string(&adapter.connection)));

    if !adapter.options.is_empty() {
        output.push_str(&format!("{}options: {{\n", indent_str(indent + 1)));
        for (key, value) in &adapter.options {
            output.push_str(&format!("{}\"{}\": \"{}\"\n", indent_str(indent + 2), escape_string(key), escape_string(value)));
        }
        output.push_str(&format!("{}}}\n", indent_str(indent + 1)));
    }

    output.push_str(&format!("{}}}\n", ind));
    output
}

fn pretty_print_adapter_type(t: &AdapterType) -> &'static str {
    match t {
        AdapterType::Postgres => "postgres",
        AdapterType::Redis => "redis",
        AdapterType::Memory => "memory",
    }
}

fn pretty_print_memory(memory: &MemoryDef, indent: usize) -> String {
    let mut output = String::new();
    let ind = indent_str(indent);

    output.push_str(&format!("{}memory {} {{\n", ind, memory.name));
    output.push_str(&format!("{}type: {}\n", indent_str(indent + 1), pretty_print_memory_type(&memory.memory_type)));

    if !memory.schema.is_empty() {
        output.push_str(&format!("{}schema: {{\n", indent_str(indent + 1)));
        for field in &memory.schema {
            let mut line = format!(
                "{}{}: {}",
                indent_str(indent + 2),
                field.name,
                pretty_print_field_type(&field.field_type)
            );
            if field.nullable {
                line.push_str(" optional");
            }
            if let Some(default) = &field.default {
                line.push_str(&format!(" = {}", default));
            }
            line.push('\n');
            output.push_str(&line);
        }
        output.push_str(&format!("{}}}\n", indent_str(indent + 1)));
    }

    output.push_str(&format!("{}retention: {}\n", indent_str(indent + 1), pretty_print_retention(&memory.retention)));
    output.push_str(&format!("{}lifecycle: {}\n", indent_str(indent + 1), pretty_print_lifecycle(&memory.lifecycle)));

    if let Some(parent) = &memory.parent {
        output.push_str(&format!("{}parent: {}\n", indent_str(indent + 1), parent));
    }

    if !memory.indexes.is_empty() {
        output.push_str(&format!("{}index: {{\n", indent_str(indent + 1)));
        for idx in &memory.indexes {
            if idx.options.is_empty() {
                output.push_str(&format!(
                    "{}{}: {}\n",
                    indent_str(indent + 2),
                    idx.field,
                    pretty_print_index_type(&idx.index_type)
                ));
            } else {
                output.push_str(&format!(
                    "{}{}: {} options: {{\n",
                    indent_str(indent + 2),
                    idx.field,
                    pretty_print_index_type(&idx.index_type)
                ));
                for (key, value) in &idx.options {
                    output.push_str(&format!(
                        "{}\"{}\": \"{}\"\n",
                        indent_str(indent + 3),
                        escape_string(key),
                        escape_string(value)
                    ));
                }
                output.push_str(&format!("{}}}\n", indent_str(indent + 2)));
            }
        }
        output.push_str(&format!("{}}}\n", indent_str(indent + 1)));
    }

    if !memory.inject_on.is_empty() {
        output.push_str(&format!("{}inject_on: [", indent_str(indent + 1)));
        let triggers: Vec<String> = memory.inject_on.iter().map(pretty_print_trigger).collect();
        output.push_str(&triggers.join(", "));
        output.push_str("]\n");
    }

    if !memory.artifacts.is_empty() {
        output.push_str(&format!("{}artifacts: [", indent_str(indent + 1)));
        let arts: Vec<String> = memory.artifacts.iter().map(|a| format!("\"{}\"", escape_string(a))).collect();
        output.push_str(&arts.join(", "));
        output.push_str("]\n");
    }

    if !memory.modifiers.is_empty() {
        output.push_str(&format!("{}modifiers: [", indent_str(indent + 1)));
        let mods: Vec<String> = memory.modifiers.iter().map(pretty_print_modifier).collect();
        output.push_str(&mods.join(", "));
        output.push_str("]\n");
    }

    output.push_str(&format!("{}}}\n", ind));
    output
}

fn pretty_print_memory_type(t: &MemoryType) -> &'static str {
    match t {
        MemoryType::Ephemeral => "ephemeral",
        MemoryType::Working => "working",
        MemoryType::Episodic => "episodic",
        MemoryType::Semantic => "semantic",
        MemoryType::Procedural => "procedural",
        MemoryType::Meta => "meta",
    }
}

fn pretty_print_field_type(t: &FieldType) -> String {
    match t {
        FieldType::Uuid => "uuid".to_string(),
        FieldType::Text => "text".to_string(),
        FieldType::Int => "int".to_string(),
        FieldType::Float => "float".to_string(),
        FieldType::Bool => "bool".to_string(),
        FieldType::Timestamp => "timestamp".to_string(),
        FieldType::Json => "json".to_string(),
        FieldType::Embedding(Some(dim)) => format!("embedding({})", dim),
        FieldType::Embedding(None) => "embedding".to_string(),
        FieldType::Enum(variants) => {
            let vars: Vec<String> = variants.iter().map(|v| format!("\"{}\"", escape_string(v))).collect();
            format!("enum({})", vars.join(", "))
        }
        FieldType::Array(inner) => format!("[{}]", pretty_print_field_type(inner)),
    }
}

fn pretty_print_retention(r: &Retention) -> String {
    match r {
        Retention::Persistent => "persistent".to_string(),
        Retention::Session => "session".to_string(),
        Retention::Scope => "scope".to_string(),
        Retention::Duration(d) => d.clone(),
        Retention::Max(n) => n.to_string(),
    }
}

fn pretty_print_lifecycle(l: &Lifecycle) -> String {
    match l {
        Lifecycle::Explicit => "explicit".to_string(),
        Lifecycle::AutoClose(trigger) => format!("auto_close({})", pretty_print_trigger(trigger)),
    }
}

fn pretty_print_trigger(t: &Trigger) -> String {
    match t {
        Trigger::TaskStart => "task_start".to_string(),
        Trigger::TaskEnd => "task_end".to_string(),
        Trigger::ScopeClose => "scope_close".to_string(),
        Trigger::TurnEnd => "turn_end".to_string(),
        Trigger::Manual => "manual".to_string(),
        Trigger::Schedule(cron) => format!("schedule(\"{}\")", escape_string(cron)),
    }
}

fn pretty_print_index_type(t: &IndexType) -> &'static str {
    match t {
        IndexType::Btree => "btree",
        IndexType::Hash => "hash",
        IndexType::Gin => "gin",
        IndexType::Hnsw => "hnsw",
        IndexType::Ivfflat => "ivfflat",
    }
}

fn pretty_print_policy(policy: &PolicyDef, indent: usize) -> String {
    let mut output = String::new();
    let ind = indent_str(indent);

    output.push_str(&format!("{}policy {} {{\n", ind, policy.name));

    for rule in &policy.rules {
        output.push_str(&format!("{}on {}: [\n", indent_str(indent + 1), pretty_print_trigger(&rule.trigger)));
        for action in &rule.actions {
            output.push_str(&format!("{}{}\n", indent_str(indent + 2), pretty_print_action(action)));
        }
        output.push_str(&format!("{}]\n", indent_str(indent + 1)));
    }

    output.push_str(&format!("{}}}\n", ind));
    output
}

fn pretty_print_action(action: &Action) -> String {
    match action {
        Action::Summarize(target) => format!("summarize({})", target),
        Action::ExtractArtifacts(target) => format!("extract_artifacts({})", target),
        Action::Checkpoint(target) => format!("checkpoint({})", target),
        Action::Prune { target, criteria } => format!("prune({}, {})", target, pretty_print_filter_expr(criteria)),
        Action::Notify(channel) => format!("notify(\"{}\")", escape_string(channel)),
        Action::Inject { target, mode } => format!("inject({}, {})", target, pretty_print_injection_mode(mode)),
        // Battle Intel Feature 4: Auto-summarization action
        Action::AutoSummarize { source_level, target_level, create_edges } => {
            format!("auto_summarize({}, {}, create_edges: {})",
                pretty_print_abstraction_level(*source_level),
                pretty_print_abstraction_level(*target_level),
                create_edges)
        }
    }
}

fn pretty_print_injection(injection: &InjectionDef, indent: usize) -> String {
    let mut output = String::new();
    let ind = indent_str(indent);

    output.push_str(&format!("{}inject {} into {} {{\n", ind, injection.source, injection.target));
    output.push_str(&format!("{}mode: {}\n", indent_str(indent + 1), pretty_print_injection_mode(&injection.mode)));
    output.push_str(&format!("{}priority: {}\n", indent_str(indent + 1), injection.priority));

    if let Some(max_tokens) = injection.max_tokens {
        output.push_str(&format!("{}max_tokens: {}\n", indent_str(indent + 1), max_tokens));
    }

    if let Some(filter) = &injection.filter {
        output.push_str(&format!("{}filter: {}\n", indent_str(indent + 1), pretty_print_filter_expr(filter)));
    }

    output.push_str(&format!("{}}}\n", ind));
    output
}

fn pretty_print_injection_mode(mode: &InjectionMode) -> String {
    match mode {
        InjectionMode::Full => "full".to_string(),
        InjectionMode::Summary => "summary".to_string(),
        InjectionMode::TopK(k) => format!("top_k({})", k),
        InjectionMode::Relevant(threshold) => format!("relevant({})", threshold),
    }
}

fn pretty_print_filter_expr(expr: &FilterExpr) -> String {
    match expr {
        FilterExpr::Comparison { field, op, value } => {
            format!("{} {} {}", field, pretty_print_compare_op(op), pretty_print_filter_value(value))
        }
        FilterExpr::And(exprs) => {
            let parts: Vec<String> = exprs.iter().map(pretty_print_filter_expr).collect();
            format!("({})", parts.join(" and "))
        }
        FilterExpr::Or(exprs) => {
            let parts: Vec<String> = exprs.iter().map(pretty_print_filter_expr).collect();
            format!("({})", parts.join(" or "))
        }
        FilterExpr::Not(inner) => format!("not {}", pretty_print_filter_expr(inner)),
    }
}

fn pretty_print_compare_op(op: &CompareOp) -> &'static str {
    match op {
        CompareOp::Eq => "=",
        CompareOp::Ne => "!=",
        CompareOp::Gt => ">",
        CompareOp::Lt => "<",
        CompareOp::Ge => ">=",
        CompareOp::Le => "<=",
        CompareOp::Contains => "contains",
        CompareOp::Regex => "~",
        CompareOp::In => "in",
    }
}

fn pretty_print_filter_value(value: &FilterValue) -> String {
    match value {
        FilterValue::String(s) => format!("\"{}\"", escape_string(s)),
        FilterValue::Number(n) => n.to_string(),
        FilterValue::Bool(b) => b.to_string(),
        FilterValue::Null => "null".to_string(),
        FilterValue::CurrentTrajectory => "current_trajectory".to_string(),
        FilterValue::CurrentScope => "current_scope".to_string(),
        FilterValue::Now => "now".to_string(),
        FilterValue::Array(values) => {
            let parts: Vec<String> = values.iter().map(pretty_print_filter_value).collect();
            format!("[{}]", parts.join(", "))
        }
    }
}

// ============================================================================
// DSL-FIRST ARCHITECTURE: Pretty Printers
// ============================================================================

/// Pretty print a trajectory definition.
fn pretty_print_trajectory(t: &TrajectoryDef, indent: usize) -> String {
    let ind = indent_str(indent);
    let inner_ind = indent_str(indent + 1);
    let mut result = format!("{}trajectory \"{}\" {{\n", ind, escape_string(&t.name));
    if let Some(desc) = &t.description {
        result.push_str(&format!("{}description: \"{}\"\n", inner_ind, escape_string(desc)));
    }
    result.push_str(&format!("{}agent_type: \"{}\"\n", inner_ind, escape_string(&t.agent_type)));
    result.push_str(&format!("{}token_budget: {}\n", inner_ind, t.token_budget));
    if !t.memory_refs.is_empty() {
        result.push_str(&format!("{}memory_refs: [{}]\n", inner_ind,
            t.memory_refs.iter().map(|r| r.as_str()).collect::<Vec<_>>().join(", ")));
    }
    result.push_str(&format!("{}}}\n", ind));
    result
}

/// Pretty print an agent definition.
fn pretty_print_agent(a: &AgentDef, indent: usize) -> String {
    let ind = indent_str(indent);
    let inner_ind = indent_str(indent + 1);
    let constraint_ind = indent_str(indent + 2);
    let mut result = format!("{}agent \"{}\" {{\n", ind, escape_string(&a.name));

    if !a.capabilities.is_empty() {
        result.push_str(&format!("{}capabilities: [{}]\n", inner_ind,
            a.capabilities.iter().map(|c| format!("\"{}\"", escape_string(c))).collect::<Vec<_>>().join(", ")));
    }

    result.push_str(&format!("{}constraints: {{\n", inner_ind));
    result.push_str(&format!("{}max_concurrent: {}\n", constraint_ind, a.constraints.max_concurrent));
    result.push_str(&format!("{}timeout_ms: {}\n", constraint_ind, a.constraints.timeout_ms));
    result.push_str(&format!("{}}}\n", inner_ind));

    result.push_str(&format!("{}permissions: {{\n", inner_ind));
    if !a.permissions.read.is_empty() {
        result.push_str(&format!("{}read: [{}]\n", constraint_ind,
            a.permissions.read.join(", ")));
    }
    if !a.permissions.write.is_empty() {
        result.push_str(&format!("{}write: [{}]\n", constraint_ind,
            a.permissions.write.join(", ")));
    }
    if !a.permissions.lock.is_empty() {
        result.push_str(&format!("{}lock: [{}]\n", constraint_ind,
            a.permissions.lock.join(", ")));
    }
    result.push_str(&format!("{}}}\n", inner_ind));

    result.push_str(&format!("{}}}\n", ind));
    result
}

/// Pretty print a cache definition.
fn pretty_print_cache(c: &CacheDef, indent: usize) -> String {
    let ind = indent_str(indent);
    let inner_ind = indent_str(indent + 1);
    let freshness_ind = indent_str(indent + 2);
    let mut result = format!("{}cache {{\n", ind);

    result.push_str(&format!("{}backend: {}\n", inner_ind, pretty_print_cache_backend(&c.backend)));
    if let Some(path) = &c.path {
        result.push_str(&format!("{}path: \"{}\"\n", inner_ind, escape_string(path)));
    }
    result.push_str(&format!("{}size_mb: {}\n", inner_ind, c.size_mb));

    match &c.default_freshness {
        FreshnessDef::BestEffort { max_staleness } => {
            result.push_str(&format!("{}default_freshness: best_effort {{\n", inner_ind));
            result.push_str(&format!("{}max_staleness: {}\n", freshness_ind, max_staleness));
            result.push_str(&format!("{}}}\n", inner_ind));
        }
        FreshnessDef::Strict => {
            result.push_str(&format!("{}default_freshness: strict\n", inner_ind));
        }
    }

    if let Some(max_entries) = c.max_entries {
        result.push_str(&format!("{}max_entries: {}\n", inner_ind, max_entries));
    }
    if let Some(ttl) = &c.ttl {
        result.push_str(&format!("{}ttl: {}\n", inner_ind, ttl));
    }

    result.push_str(&format!("{}}}\n", ind));
    result
}

fn pretty_print_cache_backend(b: &CacheBackendType) -> &'static str {
    match b {
        CacheBackendType::Lmdb => "lmdb",
        CacheBackendType::Memory => "memory",
    }
}

/// Pretty print a provider definition.
fn pretty_print_provider(p: &ProviderDef, indent: usize) -> String {
    let ind = indent_str(indent);
    let inner_ind = indent_str(indent + 1);
    let mut result = format!("{}provider \"{}\" {{\n", ind, escape_string(&p.name));

    result.push_str(&format!("{}type: {}\n", inner_ind, pretty_print_provider_type(&p.provider_type)));
    result.push_str(&format!("{}api_key: {}\n", inner_ind, pretty_print_env_value(&p.api_key)));
    result.push_str(&format!("{}model: \"{}\"\n", inner_ind, escape_string(&p.model)));

    if !p.options.is_empty() {
        result.push_str(&format!("{}options: {{\n", inner_ind));
        for (key, value) in &p.options {
            result.push_str(&format!("{}\"{}\": \"{}\"\n", indent_str(indent + 2),
                escape_string(key), escape_string(value)));
        }
        result.push_str(&format!("{}}}\n", inner_ind));
    }

    result.push_str(&format!("{}}}\n", ind));
    result
}

fn pretty_print_provider_type(t: &ProviderType) -> &'static str {
    match t {
        ProviderType::OpenAI => "openai",
        ProviderType::Anthropic => "anthropic",
        ProviderType::Custom => "custom",
    }
}

fn pretty_print_env_value(v: &EnvValue) -> String {
    match v {
        EnvValue::Env(var) => format!("env(\"{}\")", escape_string(var)),
        EnvValue::Literal(s) => format!("\"{}\"", escape_string(s)),
    }
}

/// Pretty print a modifier.
fn pretty_print_modifier(m: &ModifierDef) -> String {
    match m {
        ModifierDef::Embeddable { provider } => {
            format!("embeddable {{ provider: \"{}\" }}", escape_string(provider))
        }
        ModifierDef::Summarizable { style, on_triggers } => {
            let style_str = match style {
                SummaryStyle::Brief => "brief",
                SummaryStyle::Detailed => "detailed",
            };
            if on_triggers.is_empty() {
                format!("summarizable {{ style: {} }}", style_str)
            } else {
                let triggers: Vec<String> = on_triggers.iter().map(pretty_print_trigger).collect();
                format!("summarizable {{ style: {}, on: [{}] }}", style_str, triggers.join(", "))
            }
        }
        ModifierDef::Lockable { mode } => {
            let mode_str = match mode {
                LockMode::Exclusive => "exclusive",
                LockMode::Shared => "shared",
            };
            format!("lockable {{ mode: {} }}", mode_str)
        }
    }
}

pub(crate) fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\t', "\\t")
        .replace('\r', "\\r")
}

// ============================================================================
// CONVENIENCE FUNCTIONS
// ============================================================================

/// Parse DSL source code into an AST.
pub fn parse(source: &str) -> Result<CaliberAst, ParseError> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    parser.parse()
}

/// Parse and pretty-print DSL source code (for round-trip testing).
pub fn round_trip(source: &str) -> Result<String, ParseError> {
    let ast = parse(source)?;
    Ok(pretty_print(&ast))
}


// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Lexer Tests
    // ========================================================================

    #[test]
    fn test_lexer_keywords() {
        let source = "caliber memory policy adapter inject into on context";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert!(matches!(tokens[0].kind, TokenKind::Caliber));
        assert!(matches!(tokens[1].kind, TokenKind::Memory));
        assert!(matches!(tokens[2].kind, TokenKind::Policy));
        assert!(matches!(tokens[3].kind, TokenKind::Adapter));
        assert!(matches!(tokens[4].kind, TokenKind::Inject));
        assert!(matches!(tokens[5].kind, TokenKind::Into));
        assert!(matches!(tokens[6].kind, TokenKind::On));
        assert!(matches!(tokens[7].kind, TokenKind::Context));
    }

    #[test]
    fn test_lexer_memory_types() {
        let source = "ephemeral working episodic semantic procedural meta";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert!(matches!(tokens[0].kind, TokenKind::Ephemeral));
        assert!(matches!(tokens[1].kind, TokenKind::Working));
        assert!(matches!(tokens[2].kind, TokenKind::Episodic));
        assert!(matches!(tokens[3].kind, TokenKind::Semantic));
        assert!(matches!(tokens[4].kind, TokenKind::Procedural));
        assert!(matches!(tokens[5].kind, TokenKind::Meta));
    }

    #[test]
    fn test_lexer_operators() {
        let source = "= != > < >= <= ~ contains and or not in";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert!(matches!(tokens[0].kind, TokenKind::Eq));
        assert!(matches!(tokens[1].kind, TokenKind::Ne));
        assert!(matches!(tokens[2].kind, TokenKind::Gt));
        assert!(matches!(tokens[3].kind, TokenKind::Lt));
        assert!(matches!(tokens[4].kind, TokenKind::Ge));
        assert!(matches!(tokens[5].kind, TokenKind::Le));
        assert!(matches!(tokens[6].kind, TokenKind::Regex));
        assert!(matches!(tokens[7].kind, TokenKind::Contains));
        assert!(matches!(tokens[8].kind, TokenKind::And));
        assert!(matches!(tokens[9].kind, TokenKind::Or));
        assert!(matches!(tokens[10].kind, TokenKind::Not));
        assert!(matches!(tokens[11].kind, TokenKind::In));
    }

    #[test]
    fn test_lexer_delimiters() {
        let source = "{ } ( ) [ ] : , . ->";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert!(matches!(tokens[0].kind, TokenKind::LBrace));
        assert!(matches!(tokens[1].kind, TokenKind::RBrace));
        assert!(matches!(tokens[2].kind, TokenKind::LParen));
        assert!(matches!(tokens[3].kind, TokenKind::RParen));
        assert!(matches!(tokens[4].kind, TokenKind::LBracket));
        assert!(matches!(tokens[5].kind, TokenKind::RBracket));
        assert!(matches!(tokens[6].kind, TokenKind::Colon));
        assert!(matches!(tokens[7].kind, TokenKind::Comma));
        assert!(matches!(tokens[8].kind, TokenKind::Dot));
        assert!(matches!(tokens[9].kind, TokenKind::Arrow));
    }

    #[test]
    fn test_lexer_string_literals() {
        let source = r#""hello" "world\ntest" "escaped\"quote""#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::String("hello".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::String("world\ntest".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::String("escaped\"quote".to_string()));
    }

    #[test]
    fn test_lexer_numbers() {
        let source = "42 3.14 -10";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Number(42.0));
        assert_eq!(tokens[1].kind, TokenKind::Number(314.0 / 100.0));
        assert_eq!(tokens[2].kind, TokenKind::Number(-10.0));
    }

    #[test]
    fn test_lexer_durations() {
        let source = "30s 5m 1h 7d 2w";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Duration("30s".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Duration("5m".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::Duration("1h".to_string()));
        assert_eq!(tokens[3].kind, TokenKind::Duration("7d".to_string()));
        assert_eq!(tokens[4].kind, TokenKind::Duration("2w".to_string()));
    }

    #[test]
    fn test_lexer_comments() {
        let source = "caliber // line comment\nmemory /* block comment */ policy";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert!(matches!(tokens[0].kind, TokenKind::Caliber));
        assert!(matches!(tokens[1].kind, TokenKind::Memory));
        assert!(matches!(tokens[2].kind, TokenKind::Policy));
    }

    #[test]
    fn test_lexer_error_on_invalid_char() {
        let source = "caliber @ memory";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert!(matches!(tokens[0].kind, TokenKind::Caliber));
        assert!(matches!(tokens[1].kind, TokenKind::Error(_)));
        assert!(matches!(tokens[2].kind, TokenKind::Memory));
    }

    #[test]
    fn test_lexer_schedule_keyword() {
        let source = "schedule";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert!(matches!(tokens[0].kind, TokenKind::Schedule));
    }

    // ========================================================================
    // Parser Tests
    // ========================================================================

    fn test_parse_error(message: &str) -> ParseError {
        ParseError {
            message: message.to_string(),
            line: 0,
            column: 0,
        }
    }

    #[test]
    fn test_parse_minimal_config() -> Result<(), ParseError> {
        let source = r#"caliber: "1.0" {}"#;
        let ast = parse(source)?;

        assert_eq!(ast.version, "1.0");
        assert!(ast.definitions.is_empty());
        Ok(())
    }

    #[test]
    fn test_parse_adapter() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                adapter main_db {
                    type: postgres
                    connection: "postgresql://localhost/caliber"
                }
            }
        "#;
        let ast = parse(source)?;

        assert_eq!(ast.definitions.len(), 1);
        if let Definition::Adapter(adapter) = &ast.definitions[0] {
            assert_eq!(adapter.name, "main_db");
            assert_eq!(adapter.adapter_type, AdapterType::Postgres);
            assert_eq!(adapter.connection, "postgresql://localhost/caliber");
        } else {
            return Err(test_parse_error("Expected adapter definition"));
        }
        Ok(())
    }

    #[test]
    fn test_parse_memory() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                memory turns {
                    type: ephemeral
                    schema: {
                        id: uuid
                        content: text
                        embedding: embedding(1536)
                    }
                    retention: scope
                    lifecycle: explicit
                }
            }
        "#;
        let ast = parse(source)?;

        assert_eq!(ast.definitions.len(), 1);
        if let Definition::Memory(memory) = &ast.definitions[0] {
            assert_eq!(memory.name, "turns");
            assert_eq!(memory.memory_type, MemoryType::Ephemeral);
            assert_eq!(memory.schema.len(), 3);
            assert_eq!(memory.retention, Retention::Scope);
            assert_eq!(memory.lifecycle, Lifecycle::Explicit);
        } else {
            return Err(test_parse_error("Expected memory definition"));
        }
        Ok(())
    }

    #[test]
    fn test_parse_policy() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                policy cleanup {
                    on scope_close: [
                        summarize(turns)
                        checkpoint(scope)
                    ]
                }
            }
        "#;
        let ast = parse(source)?;

        assert_eq!(ast.definitions.len(), 1);
        if let Definition::Policy(policy) = &ast.definitions[0] {
            assert_eq!(policy.name, "cleanup");
            assert_eq!(policy.rules.len(), 1);
            assert_eq!(policy.rules[0].trigger, Trigger::ScopeClose);
            assert_eq!(policy.rules[0].actions.len(), 2);
        } else {
            return Err(test_parse_error("Expected policy definition"));
        }
        Ok(())
    }

    #[test]
    fn test_parse_injection() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                inject notes into context {
                    mode: relevant(0.8)
                    priority: 80
                    max_tokens: 2000
                    filter: category = "important"
                }
            }
        "#;
        let ast = parse(source)?;

        assert_eq!(ast.definitions.len(), 1);
        if let Definition::Injection(injection) = &ast.definitions[0] {
            assert_eq!(injection.source, "notes");
            assert_eq!(injection.target, "context");
            assert_eq!(injection.mode, InjectionMode::Relevant(0.8));
            assert_eq!(injection.priority, 80);
            assert_eq!(injection.max_tokens, Some(2000));
            assert!(injection.filter.is_some());
        } else {
            return Err(test_parse_error("Expected injection definition"));
        }
        Ok(())
    }

    #[test]
    fn test_parse_filter_expressions() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                inject notes into context {
                    mode: full
                    priority: 50
                    filter: (status = "active" and priority > 5) or category = "urgent"
                }
            }
        "#;
        let ast = parse(source)?;
        if let Definition::Injection(injection) = &ast.definitions[0] {
            assert!(injection.filter.is_some());
            // The filter should be an Or expression
            if let Some(FilterExpr::Or(_)) = &injection.filter {
                // OK
            } else {
                return Err(test_parse_error("Expected Or filter expression"));
            }
        } else {
            return Err(test_parse_error("Expected injection definition"));
        }
        Ok(())
    }

    #[test]
    fn test_parse_schedule_trigger() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                policy scheduled_cleanup {
                    on schedule("0 0 * * *"): [
                        prune(old_data, age > 30d)
                    ]
                }
            }
        "#;
        let ast = parse(source)?;

        if let Definition::Policy(policy) = &ast.definitions[0] {
            assert_eq!(policy.rules[0].trigger, Trigger::Schedule("0 0 * * *".to_string()));
        } else {
            return Err(test_parse_error("Expected policy definition"));
        }
        Ok(())
    }

    #[test]
    fn test_parse_prune_action() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                policy cleanup {
                    on task_end: [
                        prune(artifacts, age > 7d)
                    ]
                }
            }
        "#;
        let ast = parse(source)?;

        if let Definition::Policy(policy) = &ast.definitions[0] {
            if let Action::Prune { target, criteria } = &policy.rules[0].actions[0] {
                assert_eq!(target, "artifacts");
                if let FilterExpr::Comparison { field, op, .. } = criteria {
                    assert_eq!(field, "age");
                    assert_eq!(*op, CompareOp::Gt);
                } else {
                    return Err(test_parse_error("Expected comparison filter"));
                }
            } else {
                return Err(test_parse_error("Expected prune action"));
            }
        } else {
            return Err(test_parse_error("Expected policy definition"));
        }
        Ok(())
    }

    #[test]
    fn test_parse_error_line_column() -> Result<(), ParseError> {
        let source = "caliber: \"1.0\" { invalid_keyword }";
        let result = parse(source);

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.line >= 1);
        assert!(err.column >= 1);
        Ok(())
    }

    // ========================================================================
    // Pretty Printer Tests
    // ========================================================================

    #[test]
    fn test_pretty_print_minimal() {
        let ast = CaliberAst {
            version: "1.0".to_string(),
            definitions: vec![],
        };
        let output = pretty_print(&ast);
        assert!(output.contains("caliber: \"1.0\""));
    }

    #[test]
    fn test_pretty_print_adapter() {
        let ast = CaliberAst {
            version: "1.0".to_string(),
            definitions: vec![Definition::Adapter(AdapterDef {
                name: "main_db".to_string(),
                adapter_type: AdapterType::Postgres,
                connection: "postgresql://localhost/caliber".to_string(),
                options: vec![],
            })],
        };
        let output = pretty_print(&ast);
        assert!(output.contains("adapter main_db"));
        assert!(output.contains("type: postgres"));
        assert!(output.contains("connection: \"postgresql://localhost/caliber\""));
    }

    #[test]
    fn test_pretty_print_memory() {
        let ast = CaliberAst {
            version: "1.0".to_string(),
            definitions: vec![Definition::Memory(MemoryDef {
                name: "turns".to_string(),
                memory_type: MemoryType::Ephemeral,
                schema: vec![FieldDef {
                    name: "id".to_string(),
                    field_type: FieldType::Uuid,
                    nullable: false,
                    default: None,
                    security: None,
                }],
                retention: Retention::Scope,
                lifecycle: Lifecycle::Explicit,
                parent: None,
                indexes: vec![],
                inject_on: vec![],
                artifacts: vec![],
                modifiers: vec![],
            })],
        };
        let output = pretty_print(&ast);
        assert!(output.contains("memory turns"));
        assert!(output.contains("type: ephemeral"));
        assert!(output.contains("retention: scope"));
    }

    // ========================================================================
    // Round-Trip Tests
    // ========================================================================

    #[test]
    fn test_round_trip_minimal() -> Result<(), ParseError> {
        let source = r#"caliber: "1.0" {}"#;
        let ast1 = parse(source)?;
        let printed = pretty_print(&ast1);
        let ast2 = parse(&printed)?;

        assert_eq!(ast1.version, ast2.version);
        assert_eq!(ast1.definitions.len(), ast2.definitions.len());
        Ok(())
    }

    #[test]
    fn test_round_trip_adapter() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                adapter main_db {
                    type: postgres
                    connection: "postgresql://localhost/caliber"
                }
            }
        "#;
        let ast1 = parse(source)?;
        let printed = pretty_print(&ast1);
        let ast2 = parse(&printed)?;

        assert_eq!(ast1, ast2);
        Ok(())
    }

    #[test]
    fn test_round_trip_memory() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                memory turns {
                    type: ephemeral
                    schema: {
                        id: uuid
                        content: text
                    }
                    retention: scope
                    lifecycle: explicit
                }
            }
        "#;
        let ast1 = parse(source)?;
        let printed = pretty_print(&ast1);
        let ast2 = parse(&printed)?;

        assert_eq!(ast1, ast2);
        Ok(())
    }

    #[test]
    fn test_parse_defaults_and_index_options() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                memory notes {
                    type: semantic
                    schema: {
                        id: uuid
                        title: text optional = "untitled"
                        score: float = 0.75
                        active: bool = true
                    }
                    retention: persistent
                    lifecycle: explicit
                    index: {
                        embedding: hnsw options: {
                            "m": 16,
                            "ef_construction": 64
                        }
                    }
                }
            }
        "#;

        let ast = parse(source)?;
        let memory = match &ast.definitions[0] {
            Definition::Memory(def) => def,
            _ => return Err(test_parse_error("Expected memory definition")),
        };

        let title = &memory.schema[1];
        assert!(title.nullable);
        assert_eq!(title.default.as_deref(), Some("\"untitled\""));

        let score = &memory.schema[2];
        assert_eq!(score.default.as_deref(), Some("0.75"));

        let active = &memory.schema[3];
        assert_eq!(active.default.as_deref(), Some("true"));

        let index = &memory.indexes[0];
        assert_eq!(index.options.len(), 2);
        assert!(index.options.iter().any(|(k, v)| k == "m" && v == "16"));
        assert!(index.options.iter().any(|(k, v)| k == "ef_construction" && v == "64"));

        let printed = pretty_print(&ast);
        assert!(printed.contains("optional"));
        assert!(printed.contains("= \"untitled\""));
        assert!(printed.contains("options: {"));
        Ok(())
    }

    #[test]
    fn test_round_trip_policy() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                policy cleanup {
                    on scope_close: [
                        summarize(turns)
                        checkpoint(scope)
                    ]
                }
            }
        "#;
        let ast1 = parse(source)?;
        let printed = pretty_print(&ast1);
        let ast2 = parse(&printed)?;

        assert_eq!(ast1, ast2);
        Ok(())
    }

    #[test]
    fn test_round_trip_injection() -> Result<(), ParseError> {
        let source = r#"
            caliber: "1.0" {
                inject notes into context {
                    mode: full
                    priority: 50
                }
            }
        "#;
        let ast1 = parse(source)?;
        let printed = pretty_print(&ast1);
        let ast2 = parse(&printed)?;

        assert_eq!(ast1, ast2);
        Ok(())
    }
}

// ============================================================================
// PROPERTY-BASED TESTS (Task 4.10)
// ============================================================================

#[cfg(test)]
#[allow(dead_code)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    // ========================================================================
    // Property 3: DSL round-trip parsing preserves semantics
    // Feature: caliber-core-implementation, Property 3: DSL round-trip parsing preserves semantics
    // Validates: Requirements 5.8
    // ========================================================================

    // Generators for AST types
    fn arb_adapter_type() -> impl Strategy<Value = AdapterType> {
        prop_oneof![
            Just(AdapterType::Postgres),
            Just(AdapterType::Redis),
            Just(AdapterType::Memory),
        ]
    }

    fn arb_memory_type() -> impl Strategy<Value = MemoryType> {
        prop_oneof![
            Just(MemoryType::Ephemeral),
            Just(MemoryType::Working),
            Just(MemoryType::Episodic),
            Just(MemoryType::Semantic),
            Just(MemoryType::Procedural),
            Just(MemoryType::Meta),
        ]
    }

    fn arb_field_type() -> impl Strategy<Value = FieldType> {
        prop_oneof![
            Just(FieldType::Uuid),
            Just(FieldType::Text),
            Just(FieldType::Int),
            Just(FieldType::Float),
            Just(FieldType::Bool),
            Just(FieldType::Timestamp),
            Just(FieldType::Json),
            (0usize..4096).prop_map(|d| FieldType::Embedding(Some(d))),
            Just(FieldType::Embedding(None)),
        ]
    }

    fn arb_retention() -> impl Strategy<Value = Retention> {
        prop_oneof![
            Just(Retention::Persistent),
            Just(Retention::Session),
            Just(Retention::Scope),
            "[0-9]+[smhdw]".prop_map(Retention::Duration),
            (1usize..1000).prop_map(Retention::Max),
        ]
    }

    fn arb_index_type() -> impl Strategy<Value = IndexType> {
        prop_oneof![
            Just(IndexType::Btree),
            Just(IndexType::Hash),
            Just(IndexType::Gin),
            Just(IndexType::Hnsw),
            Just(IndexType::Ivfflat),
        ]
    }

    fn arb_trigger() -> impl Strategy<Value = Trigger> {
        prop_oneof![
            Just(Trigger::TaskStart),
            Just(Trigger::TaskEnd),
            Just(Trigger::ScopeClose),
            Just(Trigger::TurnEnd),
            Just(Trigger::Manual),
            // Simple cron-like patterns for schedule
            "[0-9]+ [0-9]+ \\* \\* \\*".prop_map(Trigger::Schedule),
        ]
    }

    fn arb_injection_mode() -> impl Strategy<Value = InjectionMode> {
        prop_oneof![
            Just(InjectionMode::Full),
            Just(InjectionMode::Summary),
            (1usize..100).prop_map(InjectionMode::TopK),
            (0.0f32..1.0f32).prop_map(InjectionMode::Relevant),
        ]
    }

    fn arb_compare_op() -> impl Strategy<Value = CompareOp> {
        prop_oneof![
            Just(CompareOp::Eq),
            Just(CompareOp::Ne),
            Just(CompareOp::Gt),
            Just(CompareOp::Lt),
            Just(CompareOp::Ge),
            Just(CompareOp::Le),
            Just(CompareOp::Contains),
            Just(CompareOp::In),
        ]
    }

    fn arb_filter_value() -> impl Strategy<Value = FilterValue> {
        prop_oneof![
            "[a-zA-Z0-9_]+".prop_map(FilterValue::String),
            (-1000.0f64..1000.0f64).prop_map(FilterValue::Number),
            any::<bool>().prop_map(FilterValue::Bool),
            Just(FilterValue::Null),
            Just(FilterValue::CurrentTrajectory),
            Just(FilterValue::CurrentScope),
            Just(FilterValue::Now),
            prop::collection::vec("[a-zA-Z0-9_]+".prop_map(FilterValue::String), 0..5)
                .prop_map(FilterValue::Array),
        ]
    }

    fn arb_compare_expr() -> impl Strategy<Value = FilterExpr> {
        (
            "[a-zA-Z_][a-zA-Z0-9_]*",
            arb_compare_op(),
            arb_filter_value(),
        )
            .prop_map(|(field, op, value)| FilterExpr::Comparison { field, op, value })
    }

    fn arb_filter_expr(depth: u32) -> BoxedStrategy<FilterExpr> {
        let leaf = arb_compare_expr().boxed();
        if depth == 0 {
            return leaf;
        }
        let recursive = prop_oneof![
            prop::collection::vec(arb_filter_expr(depth - 1), 1..4).prop_map(FilterExpr::And),
            prop::collection::vec(arb_filter_expr(depth - 1), 1..4).prop_map(FilterExpr::Or),
            arb_filter_expr(depth - 1).prop_map(|expr| FilterExpr::Not(Box::new(expr))),
        ];
        prop_oneof![leaf, recursive].boxed()
    }

    fn arb_field_def() -> impl Strategy<Value = FieldDef> {
        (
            "[a-zA-Z_][a-zA-Z0-9_]*",
            arb_field_type(),
            any::<bool>(),
        )
            .prop_map(|(name, field_type, nullable)| FieldDef {
                name,
                field_type,
                nullable,
                default: None,
                security: None,
            })
    }

    fn arb_index_def() -> impl Strategy<Value = IndexDef> {
        (
            "[a-zA-Z_][a-zA-Z0-9_]*",
            arb_index_type(),
            prop::collection::vec(("[a-zA-Z_]+", "[a-zA-Z0-9_]+"), 0..3),
        )
            .prop_map(|(field, index_type, options)| IndexDef {
                field,
                index_type,
                options,
            })
    }

    fn arb_memory_def() -> impl Strategy<Value = MemoryDef> {
        (
            "[a-zA-Z_][a-zA-Z0-9_]*",
            arb_memory_type(),
            prop::collection::vec(arb_field_def(), 1..6),
            arb_retention(),
            Just(Lifecycle::Explicit),
            prop::collection::vec(arb_index_def(), 0..3),
        )
            .prop_map(|(name, memory_type, schema, retention, lifecycle, indexes)| MemoryDef {
                name,
                memory_type,
                schema,
                retention,
                lifecycle,
                parent: None,
                indexes,
                inject_on: vec![],
                artifacts: vec![],
                modifiers: vec![],
            })
    }

    fn arb_adapter_def() -> impl Strategy<Value = AdapterDef> {
        (
            "[a-zA-Z_][a-zA-Z0-9_]*",
            arb_adapter_type(),
            "postgresql://[a-zA-Z0-9_/]+",
        )
            .prop_map(|(name, adapter_type, connection)| AdapterDef {
                name,
                adapter_type,
                connection,
                options: vec![],
            })
    }

    fn arb_definition() -> impl Strategy<Value = Definition> {
        prop_oneof![
            arb_adapter_def().prop_map(Definition::Adapter),
            arb_memory_def().prop_map(Definition::Memory),
        ]
    }

    fn arb_ast() -> impl Strategy<Value = CaliberAst> {
        prop::collection::vec(arb_definition(), 0..5).prop_map(|definitions| CaliberAst {
            version: "1.0".to_string(),
            definitions,
        })
    }

    proptest! {
        #[test]
        fn prop_round_trip_ast(ast in arb_ast()) {
            let source = pretty_print(&ast);
            let parsed = parse(&source).expect("parse failed");

            prop_assert_eq!(ast, parsed);
        }
    }
}
