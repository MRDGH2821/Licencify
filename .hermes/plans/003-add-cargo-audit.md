# Plan 003: Add `cargo audit` to CI and justfile

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 90eb384..HEAD -- .github/workflows/release.yml justfile`
> If any file changed since this plan was written, compare the "Current state"
> excerpts against the live code before proceeding; on a mismatch, treat it
> as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none (touches same CI file as plan 001, but independently)
- **Category**: security
- **Planned at**: commit `90eb384`, 2026-06-30

## Why this matters

Licencify has 14 direct dependencies including `ureq` (HTTP client fetching
from spdx.org) with transitive chains. There is no supply-chain vulnerability
checking in CI or local workflow. A compromised dependency would go unnoticed
until a release is published. Adding `cargo audit` catches known-vulnerability
issues at PR time and before release.

## Current state

- `.github/workflows/release.yml` — the current release CI has no audit step
- `justfile` — currently only has a `schema` recipe; no `audit` recipe

The release workflow is structured as:

```
jobs:
  build:     # 22-matrix build (each target)
  create-release:  # after all builds pass, creates GitHub release
```

There is no existing audit step anywhere in CI.

## Commands you will need

| Purpose             | Command                     | Expected on success    |
| ------------------- | --------------------------- | ---------------------- |
| Install cargo-audit | `cargo install cargo-audit` | exit 0                 |
| Run audit           | `cargo audit`               | exit 0 (no advisories) |
| Test                | `cargo test`                | all pass               |

Note: `cargo audit` may report advisories on the crate's dependency tree.
The plan verifies the command runs correctly — advisories found are real
findings the maintainer should address, not a plan failure.

## Scope

**In scope**:

- `.github/workflows/release.yml` — add a `cargo audit` job
- `justfile` — add an `audit` recipe

**Out of scope**:

- `.github/workflows/mega-linter.yml` — not security-audit related
- Any Rust source changes

## Steps

### Step 1: Add `audit` recipe to justfile

The current `justfile` contains a single recipe:

```
schema:
    cargo run schema -o licencify-schema.json
```

Append an `audit` recipe after it:

```
audit:
    cargo audit
```

**Verify**:

```
just audit 2>&1
```

Expected: `cargo audit` runs and completes (exit 0 indicates no advisories,
exit non-zero with advisory output is also valid — the point is the command
runs). If `cargo-audit` is not installed, install it first with
`cargo install cargo-audit`.

### Step 2: Add an audit job to the release workflow

In `.github/workflows/release.yml`, add a new job `audit` that runs before
the `build` matrix. This catches supply-chain issues before spending CI time
on 22 cross-compilation builds.

Insert this as a new top-level job **before** the `build:` job (jobs run
in parallel by default but adding a `needs` guard on the build matrix would
slow it down — instead, run audit in parallel as a safety signal):

```yaml
audit:
  name: cargo audit
  runs-on: ubuntu-latest
  steps:
    - uses: actions/checkout@9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0 # v7.0.0
    - uses: dtolnay/rust-toolchain@stable
    - name: Install cargo-audit
      run: cargo install cargo-audit
    - name: Run security audit
      run: cargo audit
```

Place this immediately after the `jobs:` line and before the `build:` job
definition. The exact pin versions to use for the actions:

- `actions/checkout`: match the existing hash `9c091bb21b7c1c1d1991bb908d89e4e9dddfe3e0 # v7.0.0`
- `dtolnay/rust-toolchain`: use `@stable` as the existing build jobs do

**Verify**: After editing, run:

```
grep -c 'cargo audit' .github/workflows/release.yml
```

Expected: `1` (the `run: cargo audit` line exists).

The workflow YAML must parse correctly. If `yamllint` is available:

```
yamllint .github/workflows/release.yml
```

Expected: exit 0.

## Test plan

No Rust tests apply. The audit job can only be tested in CI by pushing to
a branch. Local verification is:

- `cargo audit` runs from the justfile
- The YAML files parse without syntax errors

## Done criteria

- [ ] `just audit` runs cargo audit locally (or after installing `cargo-audit`)
- [ ] `.github/workflows/release.yml` contains a new `audit` job with `cargo audit`
- [ ] `grep -c 'cargo audit' .github/workflows/release.yml` returns 1
- [ ] No files outside `.github/workflows/release.yml` and `justfile` modified
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back if:

- The release workflow has been restructured (e.g., the `jobs:` key shape
  changed to a different pattern) — adapt the job insertion point
  accordingly, but verify with a fresh read.
- `cargo audit` cannot be installed due to Rust toolchain issues.

## Maintenance notes

- Keep the `actions/checkout` hash in sync with the other jobs when the
  workflow is updated.
- When new advisories appear, `cargo audit` will fail — that's working as
  intended. The maintainer should update affected dependencies and re-run.
