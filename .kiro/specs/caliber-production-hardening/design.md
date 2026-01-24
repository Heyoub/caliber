# Design Document

## Overview

This design addresses the production hardening requirements for CALIBER. The core architectural change is moving from in-memory storage to SQL-based persistence via SPI, with correct advisory lock semantics, strict configuration validation, and proper error handling throughout.

## Architecture Decisions

### AD-1: SQL-Based Storage via SPI

**Decision:** Replace all in-memory HashMap/RwLock storage with SPI-based SQL operations against the existing schema in `caliber_init.sql`.

**Rationale:**
- In-memory storage is session-local and lost on backend exit
- SQL tables are visible to all backends and persist across restarts
- The schema already exists in `caliber_init.sql` - we just need to use it
- SPI is the standard pgrx mechanism for SQL execution from extensions

**Implementation:**
```rust
// BEFORE: In-memory storage
static TRAJECTORIES: Lazy<RwLock<HashMap<Uuid, TrajectoryRecord>>> = ...;

// AFTER: SPI-based storage
fn trajectory_insert(record: &TrajectoryRecord) -> CaliberResult<()> {
    Spi::connect(|client| {
        client.update(
            "INSERT INTO caliber.trajectories (id, name, status, ...) VALUES ($1, $2, $3, ...)",
            None,
            Some(vec![
                (PgBuiltInOids::UUIDOID.oid(), record.id.into_datum()),
                (PgBuiltInOids::TEXTOID.oid(), record.name.clone().into_datum()),
                // ...
            ]),
        )?;
        Ok(())
    })
}
```

**Affected Files:** `caliber-pg/src/lib.rs`

---

### AD-2: Advisory Lock Mode Selection

**Decision:** Support both session-level and transaction-level advisory locks with explicit mode selection, using session-level as the coordination primitive.

**Rationale:**
- Current code acquires transaction locks but releases with session unlock (mismatch)
- Session locks persist until explicit release - better for long-running agent coordination
- Transaction locks auto-release at commit - useful for short critical sections
- Explicit mode selection prevents confusion

**Implementation:**
```rust
pub enum LockMode {
    Session,      // pg_try_advisory_lock / pg_advisory_unlock
    Transaction,  // pg_try_advisory_xact_lock / auto-release
}

fn lock_acquire(key: i64, mode: LockMode, timeout_ms: Option<i64>) -> CaliberResult<bool> {
    match mode {
        LockMode::Session => {
            // Use pg_try_advisory_lock for non-blocking
            // Or pg_advisory_lock with statement_timeout for blocking with timeout
        }
        LockMode::Transaction => {
            // Use pg_try_advisory_xact_lock
            // No explicit release needed
        }
    }
}
```

**Affected Files:** `caliber-pg/src/lib.rs`, `caliber-agents/src/lib.rs`

---

### AD-3: Deterministic Lock Key Hashing

**Decision:** Replace `DefaultHasher` with FNV-1a using a fixed seed for lock key generation.

**Rationale:**
- `DefaultHasher` is not stable across Rust versions or compilations
- Advisory lock keys must be deterministic for cross-session coordination
- FNV-1a is simple, fast, and produces stable results
- Fixed seed ensures same input always produces same key

**Implementation:**
```rust
const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

fn compute_lock_key(resource_id: &Uuid, lock_type: &str) -> i64 {
    let mut hash = FNV_OFFSET_BASIS;
    for byte in resource_id.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    for byte in lock_type.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash as i64
}
```

**Affected Files:** `caliber-agents/src/lib.rs`, `caliber-pg/src/lib.rs`

---

### AD-4: Message Persistence with NOTIFY

**Decision:** Store messages in SQL table, use pg_notify with message_id only, receivers query table for payload.

**Rationale:**
- Current implementation stores messages in backend memory - invisible to other sessions
- pg_notify payload has size limits and is ephemeral
- SQL storage allows any session to retrieve message by ID
- NOTIFY serves as a wake-up signal, not payload transport

**Implementation:**
```rust
fn message_send(msg: &AgentMessage) -> CaliberResult<()> {
    // 1. Insert into SQL table
    Spi::connect(|client| {
        client.update(
            "INSERT INTO caliber.messages (id, sender_id, recipient_id, ...) VALUES (...)",
            ...
        )?;
        
        // 2. Send NOTIFY with just the message ID
        let notify_sql = format!(
            "SELECT pg_notify('caliber_agent_{}', '{}')",
            msg.recipient_id, msg.id
        );
        client.update(&notify_sql, None, None)?;
        Ok(())
    })
}

fn message_get(id: Uuid) -> CaliberResult<Option<AgentMessage>> {
    // Query SQL table by ID
    Spi::connect(|client| {
        let result = client.select(
            "SELECT * FROM caliber.messages WHERE id = $1",
            ...
        )?;
        // Parse and return
    })
}
```

**Affected Files:** `caliber-pg/src/lib.rs`

---

### AD-5: Strict DSL Parsing Mode

**Decision:** Remove all default value fallbacks from DSL parser. Missing required fields produce ParseError.

**Rationale:**
- Current parser silently supplies defaults for adapter type, memory type, retention, priority
- This violates the "no hard-coded values" philosophy
- Silent defaults hide configuration errors until runtime
- Explicit errors at parse time are easier to debug

**Implementation:**
```rust
// BEFORE: Silent default
let adapter_type = self.parse_adapter_type().unwrap_or(AdapterType::OpenAI);

// AFTER: Required field
let adapter_type = self.parse_adapter_type()
    .ok_or_else(|| ParseError::MissingField {
        field: "type".to_string(),
        context: "adapter definition".to_string(),
    })?;
```

**Affected Files:** `caliber-dsl/src/lib.rs`

---

### AD-6: PCPConfig Without Default

**Decision:** Remove `Default` impl from `PCPConfig`. All fields are required.

**Rationale:**
- Current `PCPConfig::default()` supplies hard-coded thresholds
- This violates the framework philosophy - users must configure explicitly
- Missing config should be a compile-time or early runtime error

**Implementation:**
```rust
// REMOVE this:
// impl Default for PCPConfig { ... }

// PCPRuntime::new requires explicit config
impl PCPRuntime {
    pub fn new(config: PCPConfig) -> CaliberResult<Self> {
        // Validate all required fields are present
        // No defaults supplied
    }
}
```

**Affected Files:** `caliber-pcp/src/lib.rs`

---

### AD-7: EntityType Enum Expansion

**Decision:** Add `Turn`, `Lock`, and `Message` variants to `EntityType`.

**Rationale:**
- Current code uses `EntityType::Scope` for turn errors (incorrect)
- Lock and message operations have no appropriate entity type
- Accurate entity types improve error messages and observability

**Implementation:**
```rust
pub enum EntityType {
    Trajectory,
    Scope,
    Artifact,
    Note,
    Turn,      // NEW
    Lock,      // NEW
    Message,   // NEW
    Agent,
    Region,
}
```

**Affected Files:** `caliber-core/src/lib.rs`, `caliber-storage/src/lib.rs`, `caliber-pg/src/lib.rs`

---

### AD-8: Unicode-Safe String Truncation

**Decision:** Use `char_indices()` for string truncation instead of byte slicing.

**Rationale:**
- Current `extract_first_sentence` uses byte indices which can panic on non-ASCII
- Rust strings are UTF-8, byte boundaries may not be char boundaries
- `char_indices()` provides safe character-aligned positions

**Implementation:**
```rust
fn extract_first_sentence(text: &str, max_chars: usize) -> String {
    let mut end_pos = 0;
    let mut char_count = 0;
    
    for (idx, ch) in text.char_indices() {
        if char_count >= max_chars {
            break;
        }
        if ch == '.' || ch == '!' || ch == '?' {
            end_pos = idx + ch.len_utf8();
            break;
        }
        end_pos = idx + ch.len_utf8();
        char_count += 1;
    }
    
    text[..end_pos].to_string()
}
```

**Affected Files:** `caliber-pcp/src/lib.rs`

---

### AD-9: Debug Feature Flag

**Decision:** Gate debug endpoints behind `#[cfg(feature = "debug")]`.

**Rationale:**
- `caliber_debug_clear` and `caliber_debug_dump` can wipe production data
- Feature flags allow compile-time exclusion for release builds
- Standard Rust pattern for development-only functionality

**Implementation:**
```rust
#[cfg(feature = "debug")]
#[pg_extern]
fn caliber_debug_clear() -> bool {
    pgrx::warning!("DEBUG: Clearing all CALIBER storage");
    // ... implementation
}

// In Cargo.toml:
[features]
default = []
debug = []
```

**Affected Files:** `caliber-pg/src/lib.rs`, `caliber-pg/Cargo.toml`

---

### AD-10: Explicit Error Propagation

**Decision:** Replace all `.ok()`, `.unwrap()`, and silent failures with explicit error returns.

**Rationale:**
- Current code uses `.ok()` on pg_notify, hiding failures
- `RwLock::read().unwrap()` panics on poisoning
- Silent failures make debugging impossible
- All errors should propagate to caller

**Implementation:**
```rust
// BEFORE: Silent failure
Spi::run(&notify_sql).ok();

// AFTER: Explicit error
Spi::run(&notify_sql).map_err(|e| CaliberError::Agent(AgentError::MessageSendFailed {
    message_id: msg.id,
    reason: e.to_string(),
}))?;

// BEFORE: Panic on poison
let guard = STORAGE.read().unwrap();

// AFTER: Error on poison
let guard = STORAGE.read().map_err(|_| CaliberError::Storage(StorageError::LockPoisoned))?;
```

**Affected Files:** `caliber-pg/src/lib.rs`, `caliber-storage/src/lib.rs`

---

### AD-11: Access Control Enforcement

**Decision:** Check permissions before every read/write operation using `MemoryRegionConfig`.

**Rationale:**
- Current `caliber_check_access` always returns true for registered agents
- Memory regions define read/write permissions that must be enforced
- Collaborative regions require lock verification before writes

**Implementation:**
```rust
fn enforce_access(
    agent_id: Uuid,
    region_id: Uuid,
    operation: AccessOperation,
) -> CaliberResult<()> {
    let region = region_get(region_id)?
        .ok_or(CaliberError::Agent(AgentError::RegionNotFound(region_id)))?;
    
    match operation {
        AccessOperation::Read => {
            if !region.config.can_read(&agent_id) {
                return Err(CaliberError::Agent(AgentError::PermissionDenied {
                    agent_id,
                    region_id,
                    operation: "read",
                }));
            }
        }
        AccessOperation::Write => {
            if !region.config.can_write(&agent_id) {
                return Err(CaliberError::Agent(AgentError::PermissionDenied {
                    agent_id,
                    region_id,
                    operation: "write",
                }));
            }
            if region.config.requires_lock() && !agent_holds_lock(agent_id, region_id)? {
                return Err(CaliberError::Agent(AgentError::LockRequired {
                    agent_id,
                    region_id,
                }));
            }
        }
    }
    Ok(())
}
```

**Affected Files:** `caliber-pg/src/lib.rs`

---

### AD-12: SectionPriorities Persona Field

**Decision:** Add explicit `persona` field to `SectionPriorities` struct.

**Rationale:**
- Current implementation reuses `system` priority for persona sections
- Persona and system sections serve different purposes
- Full configurability requires separate priority knobs

**Implementation:**
```rust
pub struct SectionPriorities {
    pub system: i32,
    pub persona: i32,  // NEW - separate from system
    pub tools: i32,
    pub context: i32,
    pub history: i32,
    pub user: i32,
}
```

**Affected Files:** `caliber-core/src/lib.rs`, `caliber-core/src/context.rs`

---

## Data Flow Changes

### Before (In-Memory)
```
Client → pg_extern → RwLock<HashMap> → Response
                         ↓
              (Lost on backend exit)
```

### After (SQL-Persistent)
```
Client → pg_extern → SPI → SQL Tables → Response
                              ↓
                    (Persisted, shared across backends)
```

## Migration Strategy

1. **Phase 1:** Update `caliber_init()` to actually execute the bootstrap SQL
2. **Phase 2:** Replace in-memory storage functions with SPI equivalents
3. **Phase 3:** Fix advisory lock semantics (session vs transaction)
4. **Phase 4:** Add strict validation to DSL parser
5. **Phase 5:** Remove defaults from PCPConfig
6. **Phase 6:** Add missing EntityType variants
7. **Phase 7:** Fix string handling and error propagation
8. **Phase 8:** Gate debug endpoints behind feature flag

## Testing Strategy

- Unit tests for each SPI function with mock database
- Integration tests with actual Postgres instance
- Multi-session tests to verify cross-backend visibility
- Lock contention tests for advisory lock semantics
- DSL parser tests for strict mode validation
- Unicode string tests for safe truncation
