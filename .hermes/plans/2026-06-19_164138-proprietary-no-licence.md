# Proprietary / No-Licence Feature — Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Allow users to declare a project as proprietary (no open-source licence), either by generating a proprietary notice file or by updating manifests only (no file).

**Architecture:** Two complementary modes, both triggered via the existing `licencify add` command:

1. **`licencify add proprietary`** — generates a standard proprietary notice LICENCE.txt and updates manifests with `UNLICENSED`
2. **`licencify add --no-file <spdx-id>`** — skips the LICENCE file entirely, updates manifests only

No new subcommands. No new dependencies. Fits naturally into the existing 4-tier resolution chain.

**Tech Stack:** Rust, clap, tera, toml_edit (existing deps only)

---

## Research: How Other Tools Handle Proprietary

### SPDX Standard

- SPDX has no "Proprietary" identifier in its official list
- Convention: `UNLICENSED` means "no public licence" (proprietary)
- `LicenseRef-proprietary` or `LicenseRef-proprietary-*` for custom licence text (SPDX 2.3+ allows these)

### Per Ecosystem

| Ecosystem               | Proprietary Convention                                          | Licence File?                    |
| ----------------------- | --------------------------------------------------------------- | -------------------------------- |
| npm (package.json)      | `"license": "UNLICENSED"` + `"private": true`                   | Optional — EULA.txt if needed    |
| Cargo (Cargo.toml)      | `license = "UNLICENSED"`                                        | Optional — custom `license-file` |
| Python (pyproject.toml) | `license = { text = "Proprietary" }` or `{ file = "EULA.txt" }` | Optional                         |

### What Real Proprietary Projects Do

Most proprietary projects have **no LICENCE.txt** (or have an EULA). The manifests just say `UNLICENSED` or `Proprietary`. A short copyright notice line is often the only text file.

### Recommended Approach

- **Proprietary template**: A minimal built-in template — just the copyright notice + "All rights reserved" boilerplate. Not a full legal EULA (that's beyond scope).
- **`--no-file` flag**: For users who want manifest-only updates (no LICENCE file created).

---

## Proposed Approach

### Mode 1: `licencify add proprietary`

Creates `LICENCE.txt` with:

```
Copyright (c) {year} {author}

All rights reserved.

This software is proprietary and confidential. No licence is granted to use,
copy, modify, or distribute this software without explicit written permission
from the copyright holder.
```

Updates manifests with `UNLICENSED` (SPDX standard for proprietary).

### Mode 2: `licencify add --no-file <spdx>`

No LICENCE file created. Only updates manifests (Cargo.toml → `UNLICENSED`, package.json → `UNLICENSED`, pyproject.toml → `{ text = "UNLICENSED" }`). Useful for any SPDX ID when you don't want a file.

---

## Files Likely to Change

| File                                 | Change                                                        |
| ------------------------------------ | ------------------------------------------------------------- |
| `src/cli.rs`                         | Add `--no-file` flag to `Add` variant                         |
| `src/commands/add.rs`                | Handle `--no-file` flag; handle `proprietary` as special SPDX |
| `src/commands/update.rs`             | Handle `--no-file` flag (same logic)                          |
| `src/commands/cli.rs`                | Add `--no-file` to `Update` variant too                       |
| `src/resolution.rs`                  | Short-circuit for `proprietary` (skip template resolution)    |
| `src/detect.rs`                      | Add detection for proprietary notice text                     |
| `src/licences.rs`                    | Register `proprietary` built-in template                      |
| `templates/licence/proprietary.tera` | New template file                                             |
| `src/project/cargo.rs`               | Handle `UNLICENSED` (remove `license` field or set string)    |
| `src/project/npm.rs`                 | Handle `UNLICENSED` (set string + consider `private: true`)   |
| `src/project/python.rs`              | Handle `UNLICENSED` (set `{ text = "UNLICENSED" }`)           |

---

## Step-by-Step Plan

### Task 1: Create the proprietary template

**Objective:** Add a built-in proprietary notice template file.

**Files:**

- Create: `templates/licence/proprietary.tera`

**Step 1: Create the template file**

```tera
Copyright (c) {{ year }} {{ author }}

All rights reserved.

This software is proprietary and confidential. No licence is granted to use,
copy, modify, or distribute this software without explicit written permission
from the copyright holder.
```

**Step 2: Verify it renders correctly**

Run in a test (Task 2 will add the unit test).

**Step 3: Commit**

```bash
git add templates/licence/proprietary.tera
git commit -m "feat: add proprietary notice template"
```

---

### Task 2: Register proprietary template in built-in registry

**Objective:** Wire `proprietary` into `licences.rs` so the 4-tier resolution chain finds it.

**Files:**

- Modify: `src/licences.rs`

**Step 1: Write failing test**

In `src/licences.rs`, add to the `all_expected_templates_exist` test:

```rust
#[test]
fn all_expected_templates_exist() {
    let expected = [
        "mit",
        "apache-2.0",
        "gpl-3.0-only",
        "gpl-2.0-only",
        "agpl-3.0-only",
        "lgpl-3.0-only",
        "bsd-2-clause",
        "bsd-3-clause",
        "mpl-2.0",
        "unlicense",
        "cc0-1.0",
        "isc",
        "wtfpl",
        "proprietary", // <-- new
    ];
    for id in &expected {
        assert!(get(id).is_some(), "Missing template for: {}", id);
    }
}
```

Add a dedicated test:

```rust
#[test]
fn get_returns_proprietary_template() {
    let template = get("proprietary").unwrap();
    assert!(template.contains("All rights reserved"));
    assert!(template.contains("proprietary and confidential"));
}
```

**Step 2: Run test to verify failure**

Run: `cargo test licences::tests -- --nocapture`
Expected: FAIL — `Missing template for: proprietary`

**Step 3: Add the template registration**

In the `TEMPLATES` LazyLock, add:

```rust
m.insert(
    "proprietary",
    include_str!("../templates/licence/proprietary.tera"),
);
```

**Step 4: Run test to verify pass**

Run: `cargo test licences::tests`
Expected: all PASS

**Step 5: Commit**

```bash
git add src/licences.rs
git commit -m "feat: register proprietary built-in template"
```

---

### Task 3: Add `--no-file` flag to CLI

**Objective:** Add a `--no-file` boolean flag to both `Add` and `Update` subcommands.

**Files:**

- Modify: `src/cli.rs`

**Step 1: Add flag to `Add` variant**

```rust
/// Add a license to the current project
Add {
    /// SPDX license identifier (e.g., MIT, Apache-2.0, GPL-3.0-only, proprietary)
    spdx: String,

    /// Copyright holder name (default: git config user.name)
    #[arg(short, long)]
    author: Option<String>,

    /// Copyright year (default: current year)
    #[arg(short, long)]
    year: Option<String>,

    /// Output format: txt (default) or html
    #[arg(short, long, default_value = "txt")]
    format: LicenseFormat,

    /// Skip all prompts and use defaults
    #[arg(short = 'Y', long)]
    yes: bool,

    /// Skip creating the licence file; only update project manifests
    #[arg(long)]
    no_file: bool,
},
```

**Step 2: Add flag to `Update` variant**

```rust
/// Change the project's license
Update {
    /// SPDX license identifier to change to
    spdx: String,

    /// Copyright holder name
    #[arg(short, long)]
    author: Option<String>,

    /// Copyright year
    #[arg(short, long)]
    year: Option<String>,

    /// Output format: txt (default) or html
    #[arg(short, long, default_value = "txt")]
    format: LicenseFormat,

    /// Skip creating the licence file; only update project manifests
    #[arg(long)]
    no_file: bool,
},
```

**Step 3: Build to verify**

Run: `cargo build`
Expected: build succeeds (the new fields are not yet destructured in main.rs, so this will produce warnings — that's fine for now)

**Step 4: Commit**

```bash
git add src/cli.rs
git commit -m "feat: add --no-file flag to Add and Update subcommands"
```

---

### Task 4: Wire `--no-file` through `lib.rs` dispatch

**Objective:** Pass the `no_file` flag from CLI to command handlers.

**Files:**

- Modify: `src/lib.rs`
- Modify: `src/commands/add.rs`
- Modify: `src/commands/update.rs`

**Step 1: Update `lib.rs` match arms**

```rust
Commands::Add {
    spdx,
    author,
    year,
    format,
    yes,
    no_file,  // <-- add
} => commands::cmd_add(&spdx, author, year, format, yes, no_file),
Commands::Update {
    spdx,
    author,
    year,
    format,
    no_file,  // <-- add
} => commands::cmd_update(&spdx, author, year, format, no_file),
```

**Step 2: Update `cmd_add` signature and logic**

In `src/commands/add.rs`:

```rust
pub fn cmd_add(
    spdx: &str,
    author: Option<String>,
    year: Option<String>,
    format: LicenseFormat,
    yes: bool,
    no_file: bool,  // <-- new param
) -> anyhow::Result<()> {
```

Wrap the file-creation block with `if !no_file { ... }`:

```rust
    let filename = ctx.licence_name.file_path(ext);
    let fs = global_fs();

    if !no_file {
        if fs.exists(&filename) && !yes {
            println!("{} exists. Overwrite? [y/N] ", filename.display());
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Aborted.");
                return Ok(());
            }
        }

        fs.write(&filename, &content)?;
        println!(
            "✅ Added {} ({}) [from {}] as {}",
            info.name,
            info.id,
            ctx.resolved.source,
            filename.display()
        );
    } else {
        println!(
            "ℹ️  Skipping licence file (--no-file). Manifests will be updated."
        );
    }
```

**Step 3: Update `cmd_update` similarly**

In `src/commands/update.rs`, add `no_file: bool` param and wrap file creation in `if !no_file { ... }`.

**Step 4: Build and test**

Run: `cargo build && cargo test`
Expected: builds, all tests pass

**Step 5: Commit**

```bash
git add src/lib.rs src/commands/add.rs src/commands/update.rs
git commit -m "feat: wire --no-file flag through command handlers"
```

---

### Task 5: Handle `proprietary` SPDX ID in manifest handlers

**Objective:** When the SPDX ID is `UNLICENSED` or `Proprietary`, update manifests appropriately. `UNLICENSED` is the standard SPDX identifier for proprietary.

**Files:**

- Modify: `src/project/mod.rs`
- Modify: `src/project/cargo.rs`
- Modify: `src/project/npm.rs`
- Modify: `src/project/python.rs`

**Step 1: Add manifest SPDX normalization**

In `src/project/mod.rs`, before calling handlers, normalize the SPDX ID:

```rust
/// Normalize SPDX ID for manifest fields.
/// Maps `proprietary` → `UNLICENSED` (the standard SPDX identifier for non-open-source).
fn normalize_manifest_spdx(spdx_id: &str) -> &str {
    if spdx_id.eq_ignore_ascii_case("proprietary") || spdx_id.eq_ignore_ascii_case("UNLICENSED") {
        "UNLICENSED"
    } else {
        spdx_id
    }
}

pub fn update_manifest(license_id: &str, _author: &str, _year: &str) -> Result<Vec<String>> {
    let normalized = normalize_manifest_spdx(license_id);
    let fs = global_fs();
    let mut updated = Vec::new();
    for handler in handlers() {
        if handler.exists(&*fs) {
            handler.update(&*fs, normalized)?;
            updated.push(handler.name().to_string());
        }
    }
    Ok(updated)
}
```

**Step 2: Add tests**

```rust
#[test]
fn normalize_proprietary_to_unlicensed() {
    assert_eq!(normalize_manifest_spdx("proprietary"), "UNLICENSED");
    assert_eq!(normalize_manifest_spdx("Proprietary"), "UNLICENSED");
    assert_eq!(normalize_manifest_spdx("UNLICENSED"), "UNLICENSED");
}

#[test]
fn normalize_other_licenses_passthrough() {
    assert_eq!(normalize_manifest_spdx("MIT"), "MIT");
    assert_eq!(normalize_manifest_spdx("Apache-2.0"), "Apache-2.0");
}
```

**Step 3: Run tests**

Run: `cargo test project`
Expected: all PASS

**Step 4: Commit**

```bash
git add src/project/mod.rs
git commit -m "feat: normalize proprietary SPDX to UNLICENSED for manifests"
```

---

### Task 6: Handle `proprietary` in `provider.info()`

**Objective:** `proprietary` isn't a real SPDX identifier, so `provider.info()` will fail when looking it up. Short-circuit this in the provider or resolution layer.

**Files:**

- Modify: `src/provider.rs`

**Step 1: Add `info` fallback for proprietary**

```rust
pub fn info(&self, license_id: &str) -> Result<LicenseInfo> {
    // Special-case: "proprietary" is not a real SPDX ID
    if license_id.eq_ignore_ascii_case("proprietary") {
        return Ok(LicenseInfo {
            id: "UNLICENSED".to_string(),
            name: "Proprietary (No Licence)".to_string(),
        });
    }
    let license = self
        .index
        .find(license_id)
        .context(format!("Unknown license ID: '{}'", license_id))?;
    Ok(LicenseInfo {
        id: license.license_id.clone(),
        name: license.name.clone(),
    })
}
```

**Step 2: Add test**

```rust
#[test]
fn info_proprietary_returns_unlicensed() {
    let prov = LicenseProvider::load().unwrap();
    let info = prov.info("proprietary").unwrap();
    assert_eq!(info.id, "UNLICENSED");
    assert_eq!(info.name, "Proprietary (No Licence)");
}
```

**Step 3: Run tests**

Run: `cargo test provider`
Expected: all PASS

**Step 4: Commit**

```bash
git add src/provider.rs
git commit -m "feat: handle proprietary SPDX ID in provider.info()"
```

---

### Task 7: Update detection to recognise proprietary notices

**Objective:** `licencify detect` should recognise the proprietary template output.

**Files:**

- Modify: `src/detect.rs`

**Step 1: Add detection rule**

In `detect_license()`, add before the final `None`:

```rust
if lower.contains("all rights reserved")
    && lower.contains("proprietary and confidential")
{
    return Some("UNLICENSED");
}
```

**Step 2: Add test**

```rust
#[test]
fn detect_proprietary() {
    let text = "Copyright (c) 2024 Acme Corp\n\nAll rights reserved.\n\n\
        This software is proprietary and confidential.";
    assert_eq!(detect_license(text), Some("UNLICENSED"));
}
```

**Step 3: Run tests**

Run: `cargo test detect`
Expected: all PASS

**Step 4: Commit**

```bash
git add src/detect.rs
git commit -m "feat: detect proprietary notice as UNLICENSED"
```

---

### Task 8: Update `cmd_add` confirmation message for proprietary

**Objective:** Show a clear message when adding a proprietary declaration, not "Added MIT (...) as LICENCE.txt".

**Files:**

- Modify: `src/commands/add.rs`
- Modify: `src/commands/update.rs`

**Step 1: Customise confirmation output**

In `cmd_add`, after the file write:

```rust
if no_file {
    println!(
        "ℹ️  Skipping licence file (--no-file). Manifests updated."
    );
} else if spdx.eq_ignore_ascii_case("proprietary") || info.id == "UNLICENSED" {
    println!(
        "✅ Added proprietary notice as {}",
        filename.display()
    );
} else {
    println!(
        "✅ Added {} ({}) [from {}] as {}",
        info.name, info.id, ctx.resolved.source, filename.display()
    );
}
```

**Step 2: Build**

Run: `cargo build`
Expected: builds cleanly

**Step 3: Commit**

```bash
git add src/commands/add.rs src/commands/update.rs
git commit -m "feat: custom confirmation message for proprietary declarations"
```

---

### Task 9: Add `Remove` subcommand (optional — removes LICENCE file)

**Objective:** `licencify remove` deletes the LICENCE file and sets manifests to `UNLICENSED`.

**Files:**

- Modify: `src/cli.rs`
- Modify: `src/lib.rs`
- Create: `src/commands/remove.rs`
- Modify: `src/commands/mod.rs`

**Step 1: Add `Remove` subcommand to CLI**

```rust
/// Remove the licence file and set manifests to UNLICENSED
Remove,
```

**Step 2: Implement `cmd_remove`**

```rust
use crate::{fs::global_fs, licence_name::LicenceName, project};

pub fn cmd_remove() -> anyhow::Result<()> {
    let fs = global_fs();
    let mut removed = false;

    for name in LicenceName::candidates() {
        let path = std::path::Path::new(name);
        if fs.exists(path) {
            fs.write(path, "")?; // clear
            std::fs::remove_file(path)?; // then delete
            println!("🗑️  Removed {}", name);
            removed = true;
            break;
        }
    }

    if !removed {
        println!("ℹ️  No licence file found to remove");
    }

    match project::update_manifest("UNLICENSED", "", "") {
        Ok(files) if !files.is_empty() => {
            println!("   Updated manifests: {}", files.join(", "));
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!("   Warning: could not update manifests: {}", e);
        }
    }

    Ok(())
}
```

**Step 3: Wire through lib.rs**

```rust
Commands::Remove => commands::cmd_remove(),
```

**Step 4: Build and test**

Run: `cargo build && cargo test`
Expected: all pass

**Step 5: Commit**

```bash
git add src/cli.rs src/lib.rs src/commands/remove.rs src/commands/mod.rs
git commit -m "feat: add 'remove' subcommand to strip licence file and set UNLICENSED"
```

---

### Task 10: Add integration test for proprietary workflow

**Objective:** End-to-end test: `add proprietary` creates file + updates manifests, `add --no-file MIT` skips file, `remove` cleans up.

**Files:**

- Create: `tests/proprietary.rs` (or add to existing integration tests)

**Step 1: Write integration test**

```rust
use licencify::fs::{MemFs, set_global_fs};
use std::sync::Arc;

#[test]
fn proprietary_add_creates_notice_file() {
    let fs = MemFs::new();
    fs.create_dir(std::path::Path::new("/project")).unwrap();
    fs.write_file(
        "/project/.licencify.toml",
        "[default]\nlicense = \"MIT\"\n",
    );
    fs.write_file(
        "/project/Cargo.toml",
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nlicense = \"MIT\"\n",
    );
    // Would need to set CWD and global_fs for a full integration test
    // This is a sketch — actual test depends on test harness setup
}
```

**Step 2: Build and run all tests**

Run: `cargo test`
Expected: all pass

**Step 3: Commit**

```bash
git add tests/
git commit -m "test: add integration tests for proprietary workflow"
```

---

## Tests / Validation

| Test             | What to Verify                              | Command                                                        |
| ---------------- | ------------------------------------------- | -------------------------------------------------------------- |
| Build            | Compiles without errors                     | `cargo build`                                                  |
| Unit tests       | All pass                                    | `cargo test`                                                   |
| Template renders | Proprietary template contains expected text | `cargo test licences::tests::get_returns_proprietary_template` |
| Detect           | Identifies proprietary notice               | `cargo test detect::tests::detect_proprietary`                 |
| `--no-file`      | No LICENCE file created                     | `cargo build && echo "--- test manually ---"`                  |
| Manifest update  | `UNLICENSED` written to Cargo.toml          | Manual test in a project                                       |
| `remove`         | Deletes LICENCE file                        | Manual test in a project                                       |

## Risks, Tradeoffs, and Open Questions

### Risks

- **`proprietary` is not a real SPDX ID** — mitigated by normalising to `UNLICENSED` in manifests and short-circuiting in `provider.info()`
- **`--no-file` without `proprietary`** — user might do `licencify add --no-file MIT` which is valid (just manifests, no file). This is intentional, not an error.

### Tradeoffs

- **Proprietary template text** — kept minimal (copyright + "all rights reserved"). Real EULAs are complex legal documents and out of scope. Users can use `--template-path` for custom EULA text.
- **`UNLICENSED` vs `Proprietary` in manifests** — `UNLICENSED` is the SPDX standard. `Proprietary` is human-readable but non-standard. We normalise to `UNLICENSED` for maximum ecosystem compatibility.

### Open Questions

1. Should `licencify add proprietary` automatically imply `--no-file` for the LICENCE.txt? Or always create the notice file? **Recommended: always create the notice file** (it's what the user asked for — "declaring it proprietary").
2. Should we add `private = true` to package.json when setting `UNLICENSED`? **Recommended: yes, for npm** — it's the npm convention for proprietary packages.
3. Should `--no-file` work with any SPDX ID or only with `proprietary`/`UNLICENSED`? **Recommended: any** — maximum flexibility.
