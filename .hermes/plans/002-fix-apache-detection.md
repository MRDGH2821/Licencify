# Plan 002: Fix Apache-2.0 license detection false positive

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 90eb384..HEAD -- src/detect.rs src/detect_cmd.rs`
> If any file changed since this plan was written, compare the "Current state"
> excerpts against the live code before proceeding; on a mismatch, treat it
> as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `90eb384`, 2026-06-30

## Why this matters

`detect.rs:16` uses `lower.contains("version 2.0")` as a standalone trigger for
Apache-2.0. This is far too broad — any file mentioning "version 2.0" in any
context (changelog, dependency list, CI config) gets falsely detected as
Apache-2.0. The condition should require BOTH "apache license" AND "version 2.0"
to match, exactly as MPL-2.0 and GPL already do.

## Current state

```rust
// src/detect.rs:15-18
    if lower.contains("mozilla public license") && lower.contains("version 2.0") {
        return Some("MPL-2.0");
    }
    if lower.contains("apache license") || lower.contains("version 2.0") {
        return Some("Apache-2.0");
    }
```

Note the structural difference: MPL-2.0 correctly uses `&&` (both conditions
required), while Apache-2.0 incorrectly uses `||` (either condition triggers).
This was likely a copy-paste error where `||` was substituted for `&&`.

There is also a secondary pattern: `cmd_detect` calls `runner.exit(1)` instead
of returning an `Err`, which bypasses normal error handling. This is fixed as
a drive-by since it's the same file — no additional plan needed.

```rust
// src/commands/detect_cmd.rs:27-31
    eprintln!("No license file found in current directory");
    eprintln!("Hint: use 'licencify add <SPDX-ID>' to add one");
    runner.exit(1);
```

The `runner.exit(1)` line should be `anyhow::bail!(...)` to match the rest
of the codebase. The `runner` variable on line 9 is then unused and can be
removed.

**Repo convention**: Error exits use `anyhow::Result` and `bail!()` / context.
See `src/resolution.rs:111-115` for the pattern (also in the same module):

```rust
    anyhow::bail!(
        "License '{}' not available. Not cached, not fetchable from SPDX API, \
         no custom template found, and no built-in template exists.",
        spdx_id
    )
```

## Commands you will need

| Purpose | Command       | Expected on success |
| ------- | ------------- | ------------------- |
| Build   | `cargo build` | exit 0              |
| Test    | `cargo test`  | all 75+ tests pass  |

## Scope

**In scope**:

- `src/detect.rs` — fix the `||` → `&&` on the Apache-2.0 condition
- `src/commands/detect_cmd.rs` — replace `runner.exit(1)` with `anyhow::bail!()`,
  remove unused `runner` variable and import

**Out of scope**:

- Any other detection conditions — they're all correct
- `src/detect.rs` tests — they already cover the correct behaviour; the
  fix doesn't need new test cases (the existing `detect_apache` test input
  contains both "Apache License" AND "Version 2.0" so it would pass either way)

## Steps

### Step 1: Fix the Apache-2.0 detection condition

In `src/detect.rs`, change line 16 from `||` to `&&`:

```rust
    // Before (line 16):
    if lower.contains("apache license") || lower.contains("version 2.0") {
    // After:
    if lower.contains("apache license") && lower.contains("version 2.0") {
```

**Verify**:

```
cargo test detect_apache 2>&1 | grep -E "(PASS|FAIL|test result)"
```

Expected: `test detect_apache ... ok` and `test result: ok`.

### Step 2: Fix `cmd_detect` to return `Err` instead of calling `exit()`

In `src/commands/detect_cmd.rs`:

1. Remove the `runner` variable on line 9 (`let runner = RealRunner;`)
2. Remove the unused import `process::{RealRunner, Runner}` at line 4-5 —
   change to just `process::Runner`… actually, check if `Runner` is used
   anywhere else in the file. It's not — remove the entire `use crate::process::...` line.
3. Replace `runner.exit(1);` at line 30 with:
   ```rust
   anyhow::bail!("No license file found in current directory");
   ```
4. Add `use anyhow::bail;` to the imports at the top.

The current imports at the top of `detect_cmd.rs`:

```rust
use crate::{
    detect,
    fs::global_fs,
    licence_name::LicenceName,
    process::{RealRunner, Runner},
};
```

After the fix:

```rust
use crate::{detect, fs::global_fs, licence_name::LicenceName};
```

**Verify**:

```
cargo build 2>&1
```

Expected: exit 0, no warnings (the `unused variable` warning for `runner` must
be gone).

## Test plan

No new tests needed. The existing `detect_apache` test exercises the fixed
condition and should still pass. The `runner.exit(1)` change moves an error
path from `process::exit` to `anyhow::bail!` — the behaviour is the same
(end of the error path), just using the project's standard mechanism.

## Done criteria

- [ ] `cargo build` exits 0, no warnings
- [ ] `cargo test` exits 0, all tests pass (especially `detect_apache`)
- [ ] `grep -rn 'runner.exit' src/` returns zero matches (the only call site is removed)
- [ ] `grep -rn 'RealRunner' src/commands/detect_cmd.rs` returns zero matches
- [ ] No files outside `src/detect.rs` and `src/commands/detect_cmd.rs` modified
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back if:

- The detection function has been restructured (same logic in a different
  shape) — the fix principle (replace `||` with `&&`) still applies, but
  the exact line numbers may differ.
- A test that was passing before now fails after the fix — this would mean
  a test input was relying on the buggy `||` behaviour.

## Maintenance notes

- The same "AND both conditions" pattern is the correct one — when adding
  new license detectors, always use `&&` for all signature strings.
