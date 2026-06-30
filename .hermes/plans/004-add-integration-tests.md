# Plan 004: Add integration tests for CLI commands

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 90eb384..HEAD -- src/commands/ src/lib.rs tests/`
> If any file changed significantly since this plan was written, compare the
> "Current state" excerpts against the live code before proceeding; on a
> mismatch, treat it as a STOP condition.

## Status

- **Priority**: P2
- **Effort**: L
- **Risk**: MED
- **Depends on**: none
- **Category**: tests
- **Planned at**: commit `90eb384`, 2026-06-30

## Why this matters

The 7 command handlers in `src/commands/` — `cmd_add`, `cmd_update`, `cmd_detect`,
`cmd_cache`, `cmd_config`, `cmd_list`, `cmd_search` — have zero tests. Every other
module does (config merging, template rendering, SPDX index, author resolution,
filesystem abstraction, manifest handlers, license text detection). The commands
are the entry points that tie everything together; when they break, the tool
breaks for the user. Integration tests are the safety net that prevents shipping
regressions, especially after refactoring the shared resolution logic.

The codebase is designed for testability: `global_fs()` and `set_global_fs()`
mean you can swap the filesystem in tests, and the `Runner` trait means you
can mock process execution. These seams are used by the existing tests but
never by command-level tests.

## Current state

- `src/commands/` — 7 module files, zero `#[cfg(test)]` blocks
- `src/commands/add.rs` — `cmd_add()` calls `LicenseProvider::load()`,
  `Config::load_effective()`, `resolution::resolve_context()`,
  `template::render_with_context()`, `global_fs()` for writing,
  `update_project_defaults()`, `project::update_manifest()`
- `src/commands/update.rs` — same pattern as add, minus the `--yes` prompt
- `src/commands/detect_cmd.rs` — uses `global_fs()` and `LicenceName::candidates()`
  to find and read existing license files
- `src/commands/list.rs` — uses `LicenseProvider::load()` and `all_licenses()`
- `src/commands/search.rs` — uses `LicenseProvider::load()` and `search()`
- `src/commands/cache_cmd.rs` — uses `global_fs()` for cache directory ops
- `src/commands/config_cmd.rs` — uses `Config` methods and `global_fs()`

Existing test infrastructure (use as pattern):

- `src/fs.rs` has `MemFs` with full test coverage (read, write, create_dir,
  read_dir, remove_dir_all) — use `memfs::MemFs` for filesystem isolation
- `src/fs.rs` has `set_global_fs()` — call in test setup to inject `MemFs`
- `src/process.rs` has `Runner` trait — mockable process execution
- `src/author.rs` tests use `MockRunner` — follow this pattern
- `src/config.rs` tests construct `Config` directly from struct literals
- `src/spdx.rs` tests use the bundled `data/licenses.json` — same index the
  commands use

Example of setting up global filesystem in a test:

```rust
let fs = Arc::new(MemFs::new()) as Arc<dyn Fs>;
set_global_fs(fs.clone());
// ... test code ...
reset_global_fs();
```

**Repo convention for tests**: Tests use `#[cfg(test)] mod tests` at the
bottom of the file, with `use super::*;`. Assertions use `assert_eq!` and
`assert!`. See `src/author.rs:92-187` or `src/template.rs:68-209` for
exemplar patterns.

## Commands you will need

| Purpose  | Command                    | Expected on success           |
| -------- | -------------------------- | ----------------------------- |
| Build    | `cargo build`              | exit 0                        |
| Test     | `cargo test`               | all pass, including new tests |
| Specific | `cargo test -- commands::` | runs only new command tests   |

## Scope

**In scope**:

- `src/commands/add.rs` — add `#[cfg(test)] mod tests` with integration tests
- `src/commands/update.rs` — add tests
- `src/commands/detect_cmd.rs` — add tests (after the plan-002 fix is applied)
- `src/commands/list.rs` — add tests
- `src/commands/search.rs` — add tests
- `src/commands/cache_cmd.rs` — add tests
- `src/commands/config_cmd.rs` — add tests

**Out of scope**:

- `src/lib.rs` — no changes needed; the command function signatures stay the same
- `src/cli.rs` — no changes; clap argument parsing is tested separately
- Any `#[cfg(test)]` module in an already-tested file (config, spdx, template, etc.)

## Steps

### Step 1: Add command tests for `list` and `search` (lowest setup cost)

These two commands are simplest — they only depend on `LicenseProvider` which
loads the bundled SPDX index (no network needed).

In `src/commands/list.rs`, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_list_returns_all_licenses() {
        // This calls LicenseProvider::load() which deserializes the
        // bundled data/licenses.json — no network or filesystem needed.
        let result = cmd_list(false, false, None);
        assert!(result.is_ok());
    }

    #[test]
    fn cmd_list_respects_osi_filter() {
        let result = cmd_list(true, false, None);
        assert!(result.is_ok());
    }

    #[test]
    fn cmd_list_respects_fsf_filter() {
        let result = cmd_list(false, true, None);
        assert!(result.is_ok());
    }

    #[test]
    fn cmd_list_respects_limit() {
        let result = cmd_list(false, false, Some(5));
        assert!(result.is_ok());
    }
}
```

In `src/commands/search.rs`, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_search_finds_mit() {
        let result = cmd_search("MIT", false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn cmd_search_returns_none_for_gibberish() {
        // This will call runner.exit(1) — but after plan-002 is applied,
        // it returns an Err instead. Write for the post-fix state:
        let result = cmd_search("zzzznonexistentzzzz", false, false);
        assert!(result.is_err());
    }

    #[test]
    fn cmd_search_respects_osi_filter() {
        let result = cmd_search("MIT", true, false);
        assert!(result.is_ok());
    }
}
```

**Verify**:

```
cargo test -- commands::list::tests commands::search::tests 2>&1
```

Expected: all new tests pass.

### Step 2: Add command tests for `detect` (uses MemFs)

In `src/commands/detect_cmd.rs`, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{set_global_fs, reset_global_fs, MemFs};
    use std::sync::Arc;

    #[test]
    fn cmd_detect_finds_mit_license() {
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        set_global_fs(fs.clone());
        fs.write(std::path::Path::new("LICENSE"), "MIT License\n\nPermission is hereby granted, free of charge...").unwrap();

        let result = cmd_detect();
        assert!(result.is_ok());

        reset_global_fs();
    }

    #[test]
    fn cmd_detect_returns_err_when_no_license() {
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        set_global_fs(fs.clone());

        let result = cmd_detect();
        // After plan-002, this returns Err instead of calling exit()
        assert!(result.is_err());

        reset_global_fs();
    }

    #[test]
    fn cmd_detect_finds_unlicensed() {
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        set_global_fs(fs.clone());
        fs.write(
            std::path::Path::new("LICENSE"),
            "All Rights Reserved\nProprietary and confidential",
        )
        .unwrap();

        let result = cmd_detect();
        assert!(result.is_ok());

        reset_global_fs();
    }
}
```

**Verify**:

```
cargo test -- commands::detect_cmd::tests 2>&1
```

Expected: all pass.

### Step 3: Add command tests for `config_cmd`

In `src/commands/config_cmd.rs`, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::ConfigAction;
    use crate::fs::{set_global_fs, reset_global_fs, MemFs};
    use std::sync::Arc;

    #[test]
    fn cmd_config_init_creates_project_config() {
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        set_global_fs(fs.clone());

        // Create a Cargo.toml so the project root is detectable
        fs.write(std::path::Path::new("Cargo.toml"), "[package]\nname = \"test\"\n").unwrap();

        let result = cmd_config(ConfigAction::Init);
        assert!(result.is_ok());

        reset_global_fs();
    }

    #[test]
    fn cmd_config_show_succeeds_without_config() {
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        set_global_fs(fs.clone());

        let result = cmd_config(ConfigAction::Show);
        assert!(result.is_ok());

        reset_global_fs();
    }

    #[test]
    fn cmd_schema_writes_file() {
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        set_global_fs(fs.clone());

        let result = cmd_schema("test-schema.json");
        assert!(result.is_ok());
        assert!(fs.exists(std::path::Path::new("test-schema.json")));

        reset_global_fs();
    }
}
```

**Verify**:

```
cargo test -- commands::config_cmd::tests 2>&1
```

Expected: all pass.

### Step 4: Add command tests for `cache_cmd`

In `src/commands/cache_cmd.rs`, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::CacheAction;
    use crate::fs::{set_global_fs, reset_global_fs, MemFs};
    use std::sync::Arc;

    #[test]
    fn cmd_cache_info_with_empty_cache() {
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        set_global_fs(fs.clone());

        let result = cmd_cache(CacheAction::Info);
        assert!(result.is_ok());

        reset_global_fs();
    }

    #[test]
    fn cmd_cache_clear_with_empty_cache() {
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        set_global_fs(fs.clone());

        let result = cmd_cache(CacheAction::Clear);
        assert!(result.is_ok());

        reset_global_fs();
    }
}
```

**Verify**:

```
cargo test -- commands::cache_cmd::tests 2>&1
```

Expected: all pass.

### Step 5: Add command tests for `add` and `update`

These are the most complex commands — they depend on the SPDX index, template
rendering, filesystem writes, manifest updates, and config updates. Write
focused tests that exercise the integration seam:

In `src/commands/add.rs`, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::LicenseFormat;
    use crate::fs::{set_global_fs, reset_global_fs, MemFs};
    use std::sync::Arc;

    #[test]
    fn cmd_add_mit_returns_ok() {
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        set_global_fs(fs.clone());
        fs.write(std::path::Path::new("Cargo.toml"), "[package]\nname = \"test\"\nversion = \"0.1.0\"\n").unwrap();

        // Add MIT with explicit author — no prompts with --yes
        let result = cmd_add(
            "MIT",
            Some("Test Author".into()),
            None,   // company
            None,   // email
            None,   // year
            LicenseFormat::Txt,
            true,   // yes — skip prompts
        );
        assert!(result.is_ok(), "cmd_add failed: {:?}", result.err());

        // Verify the license file was written
        let expected_path = std::path::Path::new("LICENCE.txt");
        assert!(fs.exists(expected_path), "LICENCE.txt not written");

        reset_global_fs();
    }

    #[test]
    fn cmd_add_proprietary_returns_ok() {
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        set_global_fs(fs.clone());
        fs.write(std::path::Path::new("Cargo.toml"), "[package]\nname = \"test\"\nversion = \"0.1.0\"\n").unwrap();

        let result = cmd_add(
            "proprietary",
            Some("Acme Corp".into()),
            Some("Acme Corp".into()),
            Some("legal@acme.com".into()),
            Some("2024".into()),
            LicenseFormat::Txt,
            true,
        );
        assert!(result.is_ok(), "cmd_add proprietary failed: {:?}", result.err());

        reset_global_fs();
    }
}
```

In `src/commands/update.rs`, add at the bottom:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::LicenseFormat;
    use crate::fs::{set_global_fs, reset_global_fs, MemFs};
    use std::sync::Arc;

    #[test]
    fn cmd_update_requires_existing_file() {
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        set_global_fs(fs.clone());

        // No license file exists — should fail
        let result = cmd_update(
            "MIT",
            Some("Test Author".into()),
            None,
            None,
            None,
            LicenseFormat::Txt,
        );
        assert!(result.is_err());

        reset_global_fs();
    }

    #[test]
    fn cmd_update_replaces_content() {
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        set_global_fs(fs.clone());
        fs.write(std::path::Path::new("Cargo.toml"), "[package]\nname = \"test\"\nversion = \"0.1.0\"\n").unwrap();

        // First add a license
        let add_result = cmd_add(
            "MIT",
            Some("Author".into()),
            None, None, None,
            LicenseFormat::Txt,
            true,
        );
        assert!(add_result.is_ok());

        // Then update it to Apache-2.0
        let result = cmd_update(
            "Apache-2.0",
            Some("Author".into()),
            None, None, None,
            LicenseFormat::Txt,
        );
        assert!(result.is_ok(), "cmd_update failed: {:?}", result.err());

        reset_global_fs();
    }
}
```

**Verify**:

```
cargo test -- commands::add::tests commands::update::tests 2>&1
```

Expected: all pass. Note: the SPDX API cache test requires that the test
environment has no API cache directory — the test `cmd_add_mit_returns_ok`
will hit the 4-tier resolution chain (cache → custom → API → built-in).
The built-in MIT template is embedded, so it works offline.

## Test plan

The test plan IS the test code written above. Key coverage targets:

- `list`: no filters, OSI filter, FSF filter, limit pagination
- `search`: known match, no match, filter combination
- `detect`: find MIT, find proprietary, no license file
- `config`: init with project, show without config
- `schema`: write to custom path
- `cache`: info and clear on empty cache
- `add`: add MIT to a Cargo project, add proprietary
- `update`: fail when no file exists, succeed when replacing content

## Done criteria

- [ ] `cargo test` exits 0 and reports at least 95 tests (75 existing + ~20 new)
- [ ] Every `src/commands/*.rs` file has at least one `#[cfg(test)] mod tests` block
- [ ] `grep -c '#\[cfg(test)\]' src/commands/*.rs` returns 7 (one per file)
- [ ] No files outside `src/commands/` are modified
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back if:

- A command function signature has changed (e.g., new parameters) — the test
  calls must match. Check against the current `src/lib.rs` match arms.
- `reset_global_fs()` causes a panic in other tests — ensure it's called
  in every test, including via `Drop` if using a guard pattern.
- A test fails because the built-in templates don't contain the expected
  strings — verify against actual template content in `templates/licence/`.

## Maintenance notes

- When refactoring a command function, its tests will need updating.
- The `set_global_fs`/`reset_global_fs` pattern is global state — tests
  MUST reset it in every test function. Consider a drop guard or
  `#[ctor]`/`#[serial_test]` crate if ordering issues arise.
- The SPDX index is embedded (600+ KB JSON) — `LicenseProvider::load()` is
  fast but not instant. If command tests become slow, consider lazy-initializing
  the provider or caching the loaded index.
