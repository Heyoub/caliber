//! Parser implementation

use super::ast::*;
use crate::lexer::*;

                self.advance();
                Ok(Retention::Max(n))
            }
            _ => Err(self.error("Expected retention type")),
        }
    }

    /// Parse a lifecycle configuration.
    fn parse_lifecycle(&mut self) -> Result<Lifecycle, ParseError> {
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
    fn parse_trigger(&mut self) -> Result<Trigger, ParseError> {
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
    fn parse_index_def(&mut self) -> Result<IndexDef, ParseError> {
        let field = self.expect_identifier()?;
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
    fn parse_index_type(&mut self) -> Result<IndexType, ParseError> {
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
    fn parse_policy(&mut self) -> Result<PolicyDef, ParseError> {
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
    fn parse_action(&mut self) -> Result<Action, ParseError> {
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
    fn parse_injection(&mut self) -> Result<InjectionDef, ParseError> {
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
    fn parse_injection_mode(&mut self) -> Result<InjectionMode, ParseError> {
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
    fn parse_evolution(&mut self) -> Result<EvolutionDef, ParseError> {
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
    fn parse_summarization_policy(&mut self) -> Result<SummarizationPolicyDef, ParseError> {
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
    fn parse_summarization_trigger(&mut self) -> Result<SummarizationTriggerDsl, ParseError> {
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
    fn parse_abstraction_level(&mut self) -> Result<AbstractionLevelDsl, ParseError> {
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
    fn parse_bool(&mut self) -> Result<bool, ParseError> {
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
    fn parse_filter_expr(&mut self) -> Result<FilterExpr, ParseError> {
        self.parse_or_expr()
    }

    fn parse_or_expr(&mut self) -> Result<FilterExpr, ParseError> {
        let mut left = self.parse_and_expr()?;

        while self.check(&TokenKind::Or) {
            self.advance();
            let right = self.parse_and_expr()?;
            left = FilterExpr::Or(vec![left, right]);
        }

        Ok(left)
    }

    fn parse_and_expr(&mut self) -> Result<FilterExpr, ParseError> {
        let mut left = self.parse_comparison()?;

        while self.check(&TokenKind::And) {
            self.advance();
            let right = self.parse_comparison()?;
            left = FilterExpr::And(vec![left, right]);
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<FilterExpr, ParseError> {
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

    fn parse_compare_op(&mut self) -> Result<CompareOp, ParseError> {
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

    fn parse_filter_value(&mut self) -> Result<FilterValue, ParseError> {
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
    // Helper methods
    // ========================================================================

    fn current(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.pos += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.current().kind == TokenKind::Eof
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.current().kind) == std::mem::discriminant(kind)
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), ParseError> {
        if self.check(&kind) {
            self.advance();
            Ok(())
        } else {
            Err(self.error(&format!("Expected {:?}", kind)))
        }
    }

    fn expect_identifier(&mut self) -> Result<String, ParseError> {
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
    fn expect_field_name(&mut self) -> Result<String, ParseError> {
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
            _ => return Err(self.error("Expected identifier")),
        };
        self.advance();
        Ok(name)
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        match &self.current().kind {
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(s)
            }
            _ => Err(self.error("Expected string")),
        }
    }

    fn expect_number(&mut self) -> Result<f64, ParseError> {
        match self.current().kind {
            TokenKind::Number(n) => {
                self.advance();
                Ok(n)
            }
            _ => Err(self.error("Expected number")),
        }
    }

    fn expect_string_or_number(&mut self) -> Result<String, ParseError> {
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

    fn optional_comma(&mut self) {
        if self.check(&TokenKind::Comma) {
            self.advance();
        }
    }

    fn error(&self, msg: &str) -> ParseError {
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

fn escape_string(s: &str) -> String {
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

