# README Licence Badge/Section Update Feature

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** When `licencify add <spdx> --update-readme` is used, detect existing README and append a license badge + section. The behaviour can also be enabled globally via config.

**Architecture:** New `readme.rs` module handles README detection and content generation. `cmd_add`/`cmd_update` check config's `default.update_readme`, then CLI `--update-readme` overrides. Config field added to `DefaultConfig` and wired into the merge function.

**Tech Stack:** Rust, `global_fs()` abstraction, regex (already a dep), existing config merge pattern

---

## Tasks

### Task 1: Add `update_readme` to config model

**Objective:** Add `update_readme: Option<bool>` to `DefaultConfig` so users can set `update_readme = true` in project or global `licencify.toml`.

**Files:**

- Modify: `src/config.rs:24-53` (DefaultConfig struct), `src/config.rs:108-134` (merge fn)

**Step 1: Add field to DefaultConfig**

After `licence_name`:

```rust
    /// Automatically update README with license badge on add/update
    #[schemars(description = "Automatically update README with license badge")]
    pub update_readme: Option<bool>,
```

**Step 2: Wire into merge function**

Add to the `default:` merge block in `fn merge()`:

```rust
            update_readme: overriding.default.update_readme.or(base.default.update_readme),
```

**Step 3: Verify build**

Run: `cargo build`
Expected: exit 0

---

### Task 2: Add `--update-readme` flags to `Add` and `Update` CLI args

**Objective:** Add `--update-readme` / `-R` boolean flag to both subcommands. Clap auto-provides `--no-update-readme` for negation.

**Files:**

- Modify: `src/cli.rs:34-62` (Add), `src/cli.rs:64-90` (Update)

**Step 1: Add flag to Add**

After the `yes` field:

```rust
        /// Update README with license badge (if README exists)
        #[arg(long, default_value_t = false, conflicts_with = "no-update-readme")]
        update_readme: bool,
```

**Step 2: Add flag to Update**

Same pattern.

**Step 3: Verify**

Run: `cargo build`
Expected: exit 0

---

### Task 3: Create README module

**Objective:** Module to detect README files, generate badges, and update content.

**Files:**

- Create: `src/readme.rs`
- Modify: `src/lib.rs` — add `mod readme;`

**Step 1: Create module**

```rust
use crate::fs::{Fs, global_fs};
use anyhow::Result;
use std::path::Path;

/// README filenames to detect, ordered by preference.
const README_CANDIDATES: &[&str] = &[
    "README.md",
    "README.markdown",
    "README.rst",
    "README.txt",
    "README",
    "Readme.md",
    "readme.md",
];

/// Find an existing README file in the project root.
pub fn find_readme(fs: &dyn Fs) -> Option<&'static str> {
    README_CANDIDATES
        .iter()
        .copied()
        .find(|name| fs.exists(Path::new(name)))
}

/// Generate a shields.io badge URL for the given SPDX ID.
pub fn badge_url(spdx_id: &str) -> String {
    let encoded = spdx_id.replace(' ', "%20");
    format!(
        "[![License](https://img.shields.io/badge/License-{}-blue.svg)](LICENCE.txt)",
        encoded
    )
}

/// Generate the "License" section text to append to a README file.
pub fn license_section(spdx_id: &str) -> String {
    format!(
        "\n\n## License\n\nThis project is licensed under the [{}](LICENCE.txt) licence.\n",
        spdx_id
    )
}

/// Update the README with license badge and section (if README exists).
/// Returns true if the README was updated.
pub fn update_readme(spdx_id: &str) -> Result<bool> {
    let fs = global_fs();
    let readme_name = match find_readme(&*fs) {
        Some(name) => name,
        None => return Ok(false),
    };

    // Only handle markdown-style READMEs in v1
    if !readme_name.ends_with(".md") && !readme_name.ends_with(".markdown") {
        return Ok(false);
    }

    let content = fs.read_to_string(Path::new(readme_name)).unwrap_or_default();

    // Idempotency: skip if already has license section or badge
    if content.contains("## License") || content.contains("[![License]") {
        return Ok(false);
    }

    let badge = badge_url(spdx_id);
    let section = license_section(spdx_id);
    let updated = format!("{}\n\n{}{}", content.trim(), badge, section);
    fs.write(Path::new(readme_name), &updated)?;
    println!("   Updated {} with license badge", readme_name);
    Ok(true)
}
```

**Step 2: Register module**

In `src/lib.rs`, add `mod readme;`

**Step 3: Verify**

Run: `cargo build`
Expected: exit 0

---

### Task 4: Integrate into `cmd_add`

**Objective:** Read config's `update_readme` default, then check CLI `update_readme` override, call README updater.

**Files:**

- Modify: `src/commands/add.rs`

**Step 1: Accept `update_readme: bool` in `cmd_add` signature**

```rust
pub fn cmd_add(
    spdx: &str,
    author: Option<String>,
    company: Option<String>,
    email: Option<String>,
    year: Option<String>,
    format: LicenseFormat,
    yes: bool,
    update_readme: bool,
) -> anyhow::Result<()> {
```

**Step 2: Determine effective readme flag**

After the `yes` block (~line 47), after `let info = prov.info(spdx)?;`:

```rust
    // Determine effective readme flag: CLI overrides config
    let do_update_readme = update_readme
        || config
            .as_ref()
            .and_then(|c| c.default.update_readme)
            .unwrap_or(false);
```

**Step 3: Call `update_readme` after writing**

After the success printlns (~line 84), add:

```rust
    if do_update_readme {
        match crate::readme::update_readme(&info.id) {
            Ok(true) => {}
            Ok(false) => {
                if !yes {
                    println!("   README: not found or already has license section");
                }
            }
            Err(e) => {
                eprintln!("   Warning: could not update README: {}", e);
            }
        }
    }
```

**Step 4: Update call site in `lib.rs`**

Pass the `update_readme` field from clap.

**Step 5: Update existing tests**

Add `false` for the `update_readme` parameter to existing test calls.

---

### Task 5: Integrate into `cmd_update`

**Objective:** Same pattern for `update`.

**Files:**

- Modify: `src/commands/update.rs`

**Step 1-4:** Same as Task 4 but for `cmd_update`.

---

### Task 6: Add tests for `readme` module

**Objective:** Unit tests for README detection, badge gen, and update logic.

**Files:**

- Modify: `src/readme.rs`

Add `#[cfg(test)] mod tests` with:

| Test                                         | What it checks                                 |
| -------------------------------------------- | ---------------------------------------------- |
| `find_readme_returns_none_on_empty_fs`       | No README → None                               |
| `find_readme_finds_readme_md`                | `README.md` exists → found                     |
| `find_readme_prefers_md_over_txt`            | Both exist → prefers `.md`                     |
| `badge_url_contains_spdx_id`                 | Badge has license ID and shields.io            |
| `update_readme_adds_section`                 | Full flow: writes badge + section to README.md |
| `update_readme_skips_if_already_has_license` | Already has `## License` → No-op               |
| `update_readme_skips_if_no_readme`           | No README → No-op                              |
| `update_readme_skips_non_markdown`           | `README.rst` exists → No-op (v1)               |

All tests use `FsGuard` + `MemFs` for filesystem isolation.

---

### Task 7: Integration test for add + readme flag

**Objective:** Verify CLI flag and config default both work.

**Files:**

- Modify: `src/commands/add.rs`

```rust
#[test]
fn cmd_add_with_update_readme_flag_writes_badge() { ... }
#[test]
fn cmd_add_without_flag_does_not_touch_readme() { ... }
```

---

### Task 8: Verify everything

Run: `cargo build` → exit 0
Run: `cargo test` → 104+ passed (94 existing + 8 readme + 2 add readme)

---

## Files changed

| File                     | Action                                                                  |
| ------------------------ | ----------------------------------------------------------------------- |
| `src/readme.rs`          | **Create** — README detection, badge gen, section update                |
| `src/config.rs`          | **Modify** — add `update_readme: Option<bool>` to DefaultConfig + merge |
| `src/cli.rs`             | **Modify** — add `--update-readme` flag to Add + Update                 |
| `src/lib.rs`             | **Modify** — register `readme` module                                   |
| `src/commands/add.rs`    | **Modify** — pass+use `update_readme`, config default, new tests        |
| `src/commands/update.rs` | **Modify** — same as add                                                |

---

## Risks

| Risk                                    | Mitigation                                                                                                             |
| --------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| **Duplicate badges** on repeated runs   | Idempotency check: skip if `## License` or `[![License]` already in README                                             |
| **Non-md READMEs silently ignored**     | Documented v1 limitation; `find_readme` finds them but `update_readme` skips non-md                                    |
| **Config value + CLI flag interaction** | Clear precedence: CLI `--update-readme`/`--no-update-readme` overrides config; fallback to config value; default false |
| **LICENCE.txt badge link hardcoded**    | Acceptable for v1. Future: use resolved filename from context                                                          |
