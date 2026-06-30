# Plan 001: Fix release workflow binary names (`smt` → `licencify`)

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 90eb384..HEAD -- .github/workflows/release.yml`
> If the file changed since this plan was written, compare the "Current state"
> excerpts against the live code before proceeding; on a mismatch, treat it
> as a STOP condition.

## Status

- **Priority**: P1
- **Effort**: M
- **Risk**: HIGH
- **Depends on**: none
- **Category**: bug
- **Planned at**: commit `90eb384`, 2026-06-30

## Why this matters

The release workflow at `.github/workflows/release.yml` was copied from a
different project template (`smt`). Every artifact name, binary name
environment variable, and release file still says `smt` instead of `licencify`.
When a tag is pushed, the CI produces `smt-x86_64-unknown-linux-gnu` binaries,
writes them under `smt-${{ matrix.target }}` subfolders, and the release
publishes them as `smt-*`. The tool cannot be installed or distributed until
this is fixed — no one knows what `smt` is.

## Current state

- `.github/workflows/release.yml` — the sole release CI workflow
- The following identifiers must be changed from `smt` to `licencify`:
  - Env var `ASSET_BASE=smt-*` (line 19)
  - Env var `BIN_NAME=smt` (line 21)
  - Env var `BIN_NAME=smt.exe` (line 22, Windows branch)
  - Env var `ASSET_FILE=smt-*` (lines 24–25)
  - The `Check for Required Builds` step references `smt-x86_64-*` in `find` patterns (lines 190–192)
  - The comment header references "Build ${{ matrix.target }}" — the workflow name
    says "Release Binary Builds" which is fine, but verify the binary name references

Excerpt of affected lines (current):

```yaml
# .github/workflows/release.yml:18-26
- name: Set artifact base name
  run: |
    echo "ASSET_BASE=smt-${{ matrix.target }}" >> "$GITHUB_ENV"
    if [[ "${{ matrix.target }}" == *"-windows-"* ]]; then
      echo "BIN_NAME=smt.exe" >> "$GITHUB_ENV"
      echo "ASSET_FILE=smt-${{ matrix.target }}.exe" >> "$GITHUB_ENV"
    else
      echo "BIN_NAME=smt" >> "$GITHUB_ENV"
      echo "ASSET_FILE=smt-${{ matrix.target }}" >> "$GITHUB_ENV"
    fi
```

```yaml
# .github/workflows/release.yml:190-192
LINUX_X86_64=$(find release-artifacts -name "smt-x86_64-unknown-linux-gnu*" -type f 2>/dev/null | wc -l)
MACOS_X86_64=$(find release-artifacts -name "smt-x86_64-apple-darwin*" -type f 2>/dev/null | wc -l)
WINDOWS_X86_64=$(find release-artifacts -name "smt-x86_64-pc-windows-msvc*" -type f 2>/dev/null | wc -l)
```

**Repo convention**: The binary name in `Cargo.toml` is `licencify`. The
`./target/debug/licencify` binary is what gets built. All release artifacts
should be named `licencify-<target>`.

## Commands you will need

| Purpose | Command                                             | Expected on success        |
| ------- | --------------------------------------------------- | -------------------------- |
| Verify  | `grep -n 'smt' .github/workflows/release.yml`       | zero matches after fix     |
| Verify  | `grep -n 'licencify' .github/workflows/release.yml` | matches only the new names |
| Lint    | `yamllint .github/workflows/release.yml`            | exit 0 (if available)      |

No build step needed — this is a CI config change only.

## Scope

**In scope** (the only files you should modify):

- `.github/workflows/release.yml`

**Out of scope**:

- Any Rust source files — they already use `licencify` correctly
- The MegaLinter workflow, pre-commit config, or any other CI file
- Changing the tag format (`v[0-9]+.*`) — that's correct and already works

## Steps

### Step 1: Replace all `smt` binary name references with `licencify`

Edit `.github/workflows/release.yml` and replace the following strings
everywhere they appear:

| Before                                                   | After                                |
| -------------------------------------------------------- | ------------------------------------ |
| `ASSET_BASE=smt-`                                        | `ASSET_BASE=licencify-`              |
| `BIN_NAME=smt`                                           | `BIN_NAME=licencify`                 |
| `BIN_NAME=smt.exe`                                       | `BIN_NAME=licencify.exe`             |
| `ASSET_FILE=smt-`                                        | `ASSET_FILE=licencify-`              |
| Names in `find` patterns: `smt-x86_64-unknown-linux-gnu` | `licencify-x86_64-unknown-linux-gnu` |
| Names in `find` patterns: `smt-x86_64-apple-darwin`      | `licencify-x86_64-apple-darwin`      |
| Names in `find` patterns: `smt-x86_64-pc-windows-msvc`   | `licencify-x86_64-pc-windows-msvc`   |

**Verify**:

```
grep -rn 'smt' .github/workflows/release.yml
```

Expected: zero matches. Every `smt` must be replaced.

### Step 2: Verify there are no other stale template references

Check for any other strings that look like they came from the source
template and don't fit this project:

**Verify**:

```
grep -n 'smt\|smt-' .github/workflows/release.yml
```

Expected: empty (no matches).

## Test plan

No Rust tests apply to this change. The release workflow is not runnable
from a local environment. Verification is:

1. The grep commands above return no matches.
2. A human inspects the diff to confirm all renamed strings are
   syntactically consistent (the same variables are referenced in both
   definition and usage sites).

## Done criteria

- [ ] `grep -rn 'smt' .github/workflows/release.yml` returns zero matches
- [ ] The 3 `find` patterns under "Check for Required Builds" use `licencify` prefixes
- [ ] All env var assignments (`ASSET_BASE`, `BIN_NAME`, `ASSET_FILE`) use `licencify`
- [ ] No files outside `.github/workflows/release.yml` are modified (`git status`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back (do not improvise) if:

- The workflow file has changed significantly from the excerpts above
  (e.g., the release matrix was restructured).
- You find `smt` in places that are NOT simple string replacements
  (e.g., a shell function or computed variable name).
- You discover the file was replaced or deleted.

## Maintenance notes

- When adding new target triples to the matrix, copy the `licencify-` naming.
- The `create-release` job's `Check for Required Builds` step must be updated
  if the required platform set changes.
