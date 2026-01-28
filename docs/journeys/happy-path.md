# Critical Path Journey

Create trajectory → scope → artifact → close scope → complete trajectory

1. Create trajectory
   - Route: POST /api/v1/trajectories
   - Handler: caliber-api/src/routes/trajectory.rs
   - DB: DbClient::create<TrajectoryResponse>
   - Tests: tests/e2e/critical-path.e2e.test.ts, caliber-api/src/routes/trajectory.rs

2. Create scope
   - Route: POST /api/v1/scopes
   - Handler: caliber-api/src/routes/scope.rs
   - DB: DbClient::create<ScopeResponse>
   - Tests: tests/e2e/critical-path.e2e.test.ts, caliber-api/src/routes/scope.rs

3. Create artifact
   - Route: POST /api/v1/artifacts
   - Handler: caliber-api/src/routes/artifact.rs
   - DB: DbClient::create<ArtifactResponse>
   - Tests: tests/e2e/critical-path.e2e.test.ts, caliber-api/src/routes/artifact.rs

4. Close scope
   - Route: POST /api/v1/scopes/{id}/close
   - Handler: caliber-api/src/routes/scope.rs
   - DB: DbClient::scope_close
   - Tests: tests/e2e/critical-path.e2e.test.ts

5. Complete trajectory
   - Route: PATCH /api/v1/trajectories/{id}
   - Handler: caliber-api/src/routes/trajectory.rs
   - DB: DbClient::update_raw<TrajectoryResponse>
   - Tests: tests/e2e/critical-path.e2e.test.ts

