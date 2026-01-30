# WorkOS Development Setup

## TL;DR

CALIBER supports two auth flows for development:

1. **Internal JWT** (default, fast): No external deps, perfect for unit tests and quick iteration
2. **WorkOS Staging + Test SSO** (optional, realistic): Full SSO flow for e2e/manual testing

Use internal JWT by default. Use WorkOS staging when you need to test real SSO flows.

---

## Quick Start (Internal JWT)

**Default mode** - no setup required:

```bash
# Start API with internal JWT (works offline)
cargo run --bin caliber-api

# Generate test token
curl -X POST http://localhost:3000/auth/token \
  -H "Content-Type: application/json" \
  -d '{"user_id": "dev-user", "tenant_id": "...", "roles": ["admin"]}'
```

**Benefits:**
- ✅ Zero external dependencies
- ✅ Works offline
- ✅ Fast (no network calls)
- ✅ Deterministic (no clock issues)
- ✅ Perfect for CI/tests

---

## WorkOS Staging Setup (Optional)

When you need realistic SSO testing (e2e, manual flows, WorkOS integration work):

### 1. Get WorkOS Staging Credentials

Sign up at [WorkOS Dashboard](https://dashboard.workos.com) and:

1. Create a **Test Application** (uses staging environment automatically)
2. Get your credentials from **API Keys** tab:
   - `WORKOS_CLIENT_ID` (application ID)
   - `WORKOS_API_KEY` (staging secret key)
3. Set redirect URI: `http://localhost:3000/auth/callback`

### 2. Configure Test SSO (No Real IdP Required!)

WorkOS provides **Test SSO** - a fake identity provider for development:

1. Go to **Organizations** → Create organization
2. Add **Test SSO** connection (no IdP setup required!)
3. Test SSO simulates Google/Okta/SAML without external services

Reference: [WorkOS Test SSO Docs](https://workos.com/docs/sso/test-sso)

### 3. Environment Variables

```bash
# .env (local only, never commit)
CALIBER_AUTH_PROVIDER=workos
CALIBER_WORKOS_CLIENT_ID=your_client_id_here
CALIBER_WORKOS_API_KEY=wk_test_your_api_key_here
CALIBER_WORKOS_REDIRECT_URI=http://localhost:3000/auth/callback
```

### 4. Start API with WorkOS

```bash
# Load .env vars
source .env

# Start API in WorkOS mode
cargo run --bin caliber-api
```

### 5. Test SSO Flow

```bash
# Open in browser (will use Test SSO)
open http://localhost:3000/login
```

WorkOS Test SSO will prompt you to enter a fake user email. After "login", you'll be redirected back to your app with a valid session.

---

## How It Works

### Internal JWT Flow (Default)

```
┌─────────┐                    ┌──────────┐
│ Client  │──── POST /token ───▶│ API      │
│         │◀─── JWT (signed) ───│ (direct) │
└─────────┘                    └──────────┘
```

Your API generates and signs JWTs directly. No external service involved.

### WorkOS Flow (Optional)

```
┌─────────┐      ┌──────────┐      ┌──────────┐
│ Browser │─────▶│ WorkOS   │─────▶│ Test SSO │
│         │◀─────│ (OAuth)  │◀─────│ (fake)   │
└─────────┘      └──────────┘      └──────────┘
     │                                    │
     ▼                                    ▼
┌─────────┐                    ┌──────────┐
│ Client  │◀─── Your JWT ──────│ API      │
│ (authed)│      (from claims) │ (verifies│
└─────────┘                    │  WorkOS) │
                               └──────────┘
```

Even with WorkOS, **you still issue your own JWT** for API requests. WorkOS just handles the initial SSO login.

**Why issue your own JWT?**
- WorkOS tokens expire on their schedule
- Your JWT = your custom claims (tenant_id, roles, etc.)
- No network call on every API request (verify signature locally)
- Works offline after initial login

---

## CI/Testing Strategy

### Unit Tests → Internal JWT

```rust
#[test]
fn test_auth() {
    let config = AuthConfig {
        auth_provider: AuthProvider::Jwt,
        clock: Arc::new(FixedClock(1704067200)), // Deterministic
        ..Default::default()
    };
    // Test auth logic without external deps
}
```

**Why:**
- Fast (no network)
- Deterministic (no clock skew)
- Works on forks (no secrets required)
- Parallel-safe

### E2E Tests → WorkOS Staging (Nightly Only)

```bash
# .github/workflows/e2e-nightly.yml
on:
  schedule:
    - cron: '0 2 * * *'  # 2am daily

env:
  CALIBER_AUTH_PROVIDER: workos
  WORKOS_CLIENT_ID: ${{ secrets.WORKOS_STAGING_CLIENT_ID }}
  WORKOS_API_KEY: ${{ secrets.WORKOS_STAGING_API_KEY }}
```

**Why:**
- Runs on schedule (not on every PR)
- Uses GitHub Secrets (safe from forks)
- Tests real SSO integration
- Doesn't block fast feedback loop

### PR CI → Internal JWT Only

**Never** use WorkOS in PR CI:
- ❌ Secrets don't work on forks
- ❌ Network dependency = flaky tests
- ❌ Slower feedback
- ✅ Internal JWT is sufficient for auth logic tests

---

## Troubleshooting

### "WorkOS API key invalid"
- Check you're using **staging** API key (starts with `wk_test_`)
- Verify client ID matches the application in WorkOS Dashboard

### "Redirect URI mismatch"
- Ensure `CALIBER_WORKOS_REDIRECT_URI` matches Dashboard exactly
- Default: `http://localhost:3000/auth/callback`

### "Test SSO not working"
- Test SSO is a **WorkOS feature**, not a real IdP
- Just enter any email address when prompted
- It's a fake login flow for development

### "JWT expired immediately"
- Check your system clock (should be after 2024)
- In tests, use `FixedClock` to avoid clock issues
- See `caliber-api/src/auth.rs` for clock abstraction

---

## Best Practices

1. **Default to Internal JWT** - fastest iteration, works offline
2. **Use WorkOS for manual testing** - when you need realistic SSO flows
3. **Never commit .env** - secrets stay local
4. **Test SSO is your friend** - no real IdP setup needed
5. **Issue your own JWTs** - even with WorkOS, you control sessions

---

## Resources

- [WorkOS Test SSO](https://workos.com/docs/sso/test-sso)
- [WorkOS Staging Environment](https://workos.com/docs/reference)
- [CALIBER Auth Module](../caliber-api/src/auth.rs)
- [JWT Clock Abstraction](../caliber-api/src/auth.rs#L20-L73)

---

## Summary

```bash
# 99% of the time (unit tests, local dev)
cargo test
cargo run --bin caliber-api  # Uses internal JWT

# 1% of the time (manual SSO testing, e2e validation)
export CALIBER_AUTH_PROVIDER=workos
export WORKOS_CLIENT_ID=...
export WORKOS_API_KEY=...
cargo run --bin caliber-api  # Uses WorkOS staging
```

Keep it simple. Internal JWT for speed, WorkOS staging when you need it.
