# Licencify Remaining Features Plan

> **For Hermes:** Use subagent-driven-development skill to implement plan task-by-task.

**Goal:** Complete the remaining licencify features: built-in embedded templates for offline use, proper template rendering, project manifest integration (Cargo.toml/package.json/pyproject.toml), and user configuration.

**Current state:** CLI interface with 6 subcommands works. SPDX index (727 licenses) embedded. SPDX detail fetcher with disk caching functional. `add` command fetches from SPDX API and does inline string replacement for `<year>` and `<copyright holders>`.

**Architecture:** Single binary, clap CLI dispatch. Template resolution follows 3-tier chain: (1) local disk cache, (2) SPDX detail API, (3) built-in embedded templates. Project file integration is opt-in per file type.

---

## What's Built vs What's Missing

| Module            | Status     | Notes                                                     |
| ----------------- | ---------- | --------------------------------------------------------- |
| `src/cli.rs`      | ✅ Done    | 6 subcommands: add, list, search, detect, update, cache   |
| `src/spdx.rs`     | ✅ Done    | Embedded index (727 licenses), search, find               |
| `src/license.rs`  | ✅ Done    | Validation, info lookup                                   |
| `src/registry.rs` | ✅ Done    | SPDX detail fetcher with disk cache                       |
| `src/main.rs`     | ✅ Done    | Command dispatch + inline implementations                 |
| `src/template.rs` | ❌ Missing | Template rendering with placeholder interpolation         |
| `src/builtin/`    | ❌ Missing | Built-in embedded templates (MIT, Apache, GPL, BSD, etc.) |
| `src/config.rs`   | ❌ Missing | User preferences (default author, default license)        |
| `src/project/`    | ❌ Missing | Cargo.toml, package.json, pyproject.toml integration      |
| `src/cache.rs`    | ❌ Missing | Cache module (currently inline in main.rs)                |

---

## Task 1: Built-in Embedded Templates

**Objective:** Create `src/builtin/` module with embedded license text for the 14 most popular licenses, so `licencify add` works fully offline without network access.

**Files:**

- Create: `src/builtin/mod.rs`
- Create: `src/builtin/mit.rs`
- Create: `src/builtin/apache.rs`
- Create: `src/builtin/gpl3.rs`
- Create: `src/builtin/gpl2.rs`
- Create: `src/builtin/agpl3.rs`
- Create: `src/builtin/lgpl3.rs`
- Create: `src/builtin/bsd2.rs`
- Create: `src/builtin/bsd3.rs`
- Create: `src/builtin/mpl2.rs`
- Create: `src/builtin/unlicense.rs`
- Create: `src/builtin/cc01.rs`
- Create: `src/builtin/isc.rs`
- Create: `src/builtin/wtfpl.rs`

**Step 1:** Create `src/builtin/mod.rs`

```rust
mod mit;
mod apache;
mod gpl3;
mod gpl2;
mod agpl3;
mod lgpl3;
mod bsd2;
mod bsd3;
mod mpl2;
mod unlicense;
mod cc01;
mod isc;
mod wtfpl;

use std::collections::HashMap;

static BUILTIN: once_cell::sync::Lazy<HashMap<&'static str, &'static str>> =
    once_cell::sync::Lazy::new(|| {
        let mut m = HashMap::new();
        m.insert("mit", mit::TEXT);
        m.insert("apache-2.0", apache::TEXT);
        m.insert("gpl-3.0-only", gpl3::TEXT);
        m.insert("gpl-3.0-or-later", gpl3::TEXT);
        m.insert("gpl-2.0-only", gpl2::TEXT);
        m.insert("gpl-2.0-or-later", gpl2::TEXT);
        m.insert("agpl-3.0-only", agpl3::TEXT);
        m.insert("agpl-3.0-or-later", agpl3::TEXT);
        m.insert("lgpl-3.0-only", lgpl3::TEXT);
        m.insert("lgpl-3.0-or-later", lgpl3::TEXT);
        m.insert("bsd-2-clause", bsd2::TEXT);
        m.insert("bsd-3-clause", bsd3::TEXT);
        m.insert("mpl-2.0", mpl2::TEXT);
        m.insert("unlicense", unlicense::TEXT);
        m.insert("cc0-1.0", cc01::TEXT);
        m.insert("isc", isc::TEXT);
        m.insert("wtfpl", wtfpl::TEXT);
        m
    });

/// Look up built-in template by lowercase SPDX ID.
/// Returns `None` if no built-in template exists for that ID.
pub fn get(spdx_lower: &str) -> Option<&'static str> {
    BUILTIN.get(spdx_lower).copied()
}

/// Return all supported SPDX IDs with built-in templates.
pub fn supported_ids() -> Vec<&'static str> {
    let mut keys: Vec<&str> = BUILTIN.keys().copied().collect();
    keys.sort();
    keys
}
```

**Step 2:** Create template files (example: `src/builtin/mit.rs`)

```rust
pub const TEXT: &str = r#"MIT License

Copyright (c) [year] [fullname]

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE."#;
```

**Step 3:** Create remaining template files (apache.rs, gpl3.rs, etc.) with full license text using `[year]` and `[fullname]` placeholders.

**Step 4:** Add `once_cell` dependency to `Cargo.toml`

```toml
once_cell = "1"
```

**Step 5:** Verify build
Run: `cargo check`
Expected: success, warnings about unused modules.

**Step 6:** Commit

```bash
git add src/builtin/ Cargo.toml
git commit "feat: add built-in embedded templates for 14 popular licenses"
```

---

## Task 2: Template Rendering Module

**Objective:** Create `src/template.rs` that handles placeholder interpolation and renders final license text.

**Files:**

- Create: `src/template.rs`
- Modify: `src/main.rs` (use template module)

**Step 1:** Create `src/template.rs`

```rust
use anyhow::{Context, Result};

/// Render a license template by replacing placeholders.
///
/// Supported placeholders:
/// - `[year]` or `<year>` → current year or provided year
/// - `[fullname]` or `<copyright holders>` → author name
/// - `[email]` → author email (optional)
pub fn render(template: &str, year: &str, author: &str) -> String {
    template
        .replace("[year]", year)
        .replace("<year>", year)
        .replace("[fullname]", author)
        .replace("<copyright holders>", author)
}

/// Resolve the default author from git config.
pub fn default_author() -> Result<String> {
    let output = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .context("Failed to run git config")?;

    if output.status.success() {
        let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !name.is_empty() {
            return Ok(name);
        }
    }

    Err(anyhow::anyhow!(
        "Could not determine author name. Use --author to specify it."
    ))
}

/// Resolve the default year (current year).
pub fn default_year() -> String {
    chrono::Local::now().format("%Y").to_string()
}
```

**Step 2:** Update `src/main.rs` to use template module

Add `mod template;` and refactor `cmd_add` and `cmd_update` to use `template::render()`, `template::default_author()`, and `template::default_year()`.

**Step 3:** Verify build
Run: `cargo check`
Expected: success.

**Step 4:** Commit

```bash
git add src/template.rs src/main.rs
git commit "feat: add template rendering module with placeholder interpolation"
```

---

## Task 3: Cache Module Extraction

**Objective:** Extract cache logic from `main.rs` into a proper `src/cache.rs` module.

**Files:**

- Create: `src/cache.rs`
- Modify: `src/main.rs` (use cache module)

**Step 1:** Create `src/cache.rs`

```rust
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Manages local on-disk cache of fetched license templates.
pub struct LicenseCache {
    cache_dir: PathBuf,
}

impl LicenseCache {
    /// Create cache at XDG cache home / licencify / templates
    pub fn new() -> Result<Self> {
        let base = dirs::cache_dir()
            .context("Could not determine XDG cache directory")?;
        let cache_dir = base.join("licencify").join("templates");
        Ok(Self { cache_dir })
    }

    /// Path where a license (given lowercased SPDX key) is cached.
    pub fn path_for(&self, spdx_key: &str) -> PathBuf {
        self.cache_dir.join(format!("{spdx_key}.json"))
    }

    /// Try to read a cached license. Returns `None` if not cached.
    pub fn get(&self, spdx_key: &str) -> Option<String> {
        let path = self.path_for(spdx_key);
        if path.exists() {
            std::fs::read_to_string(&path).ok()
        } else {
            None
        }
    }

    /// Store a license template in the cache.
    pub fn put(&self, spdx_key: &str, content: &str) -> Result<()> {
        if let Some(parent) = self.path_for(spdx_key).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(self.path_for(spdx_key), content)
            .context("Failed to write cache file")?;
        Ok(())
    }

    /// Clear all cached templates.
    pub fn clear(&self) -> Result<usize> {
        if self.cache_dir.exists() {
            let count = std::fs::read_dir(&self.cache_dir)?
                .filter_map(|e| e.ok())
                .count();
            std::fs::remove_dir_all(&self.cache_dir)?;
            Ok(count)
        } else {
            Ok(0)
        }
    }

    /// Get cache directory path.
    pub fn dir(&self) -> &std::path::Path {
        &self.cache_dir
    }

    /// Count cached templates.
    pub fn count(&self) -> usize {
        if self.cache_dir.exists() {
            std::fs::read_dir(&self.cache_dir)
                .map(|r| r.filter_map(|e| e.ok()).count())
                .unwrap_or(0)
        } else {
            0
        }
    }
}
```

**Step 2:** Update `src/main.rs` to use `LicenseCache` instead of inline cache logic.

**Step 3:** Verify build
Run: `cargo check`
Expected: success.

**Step 4:** Commit

```bash
git add src/cache.rs src/main.rs
git commit "refactor: extract cache logic into cache.rs module"
```

---

## Task 4: Project Manifest Integration

**Objective:** Detect and update project manifest files (Cargo.toml, package.json, pyproject.toml) with the license field.

**Files:**

- Create: `src/project/mod.rs`
- Create: `src/project/cargo.rs`
- Create: `src/project/npm.rs`
- Create: `src/project/python.rs`
- Modify: `src/main.rs` (add `mod project`)

**Step 1:** Create `src/project/mod.rs`

```rust
mod cargo;
mod npm;
mod python;

use anyhow::Result;

/// Detect project type and update manifest with license.
pub fn update_manifest(license_id: &str, author: &str, year: &str) -> Result<Vec<String>> {
    let mut updated = Vec::new();

    if cargo::exists() {
        cargo::update(license_id)?;
        updated.push("Cargo.toml".to_string());
    }

    if npm::exists() {
        npm::update(license_id, author)?;
        updated.push("package.json".to_string());
    }

    if python::exists() {
        python::update(license_id)?;
        updated.push("pyproject.toml".to_string());
    }

    Ok(updated)
}
```

**Step 2:** Create `src/project/cargo.rs`

```rust
use anyhow::Result;
use std::fs;

pub fn exists() -> bool {
    std::path::Path::new("Cargo.toml").exists()
}

pub fn update(license_id: &str) -> Result<()> {
    let content = fs::read_to_string("Cargo.toml")?;
    let mut doc: toml_edit::DocumentMut = content.parse()?;

    if let Some(package) = doc.get_mut("package") {
        if let Some(pkg) = package.as_table_like_mut() {
            pkg.insert("license", toml_edit::value(license_id));
        }
    }

    fs::write("Cargo.toml", doc.to_string())?;
    Ok(())
}
```

**Step 3:** Create `src/project/npm.rs`

```rust
use anyhow::Result;
use serde_json::{json, Value};
use std::fs;

pub fn exists() -> bool {
    std::path::Path::new("package.json").exists()
}

pub fn update(license_id: &str, _author: &str) -> Result<()> {
    let content = fs::read_to_string("package.json")?;
    let mut pkg: Value = serde_json::from_str(&content)?;

    pkg["license"] = json!(license_id);

    let output = serde_json::to_string_pretty(&pkg)?;
    fs::write("package.json", output)?;
    Ok(())
}
```

**Step 4:** Create `src/project/python.rs`

```rust
use anyhow::Result;
use std::fs;

pub fn exists() -> bool {
    std::path::Path::new("pyproject.toml").exists()
}

pub fn update(license_id: &str) -> Result<()> {
    let content = fs::read_to_string("pyproject.toml")?;
    let mut doc: toml_edit::DocumentMut = content.parse()?;

    if let Some(project) = doc.get_mut("project") {
        if let Some(proj) = project.as_table_like_mut() {
            proj.insert("license", toml_edit::value(license_id));
        }
    }

    fs::write("pyproject.toml", doc.to_string())?;
    Ok(())
}
```

**Step 5:** Add `toml_edit` dependency to `Cargo.toml`

```toml
toml_edit = "0.22"
```

**Step 6:** Update `src/main.rs` to add `mod project;` and call `project::update_manifest()` in `cmd_add` and `cmd_update`.

**Step 7:** Verify build
Run: `cargo check`
Expected: success.

**Step 8:** Commit

```bash
git add src/project/ src/main.rs Cargo.toml
git commit "feat: add project manifest integration (Cargo.toml, package.json, pyproject.toml)"
```

---

## Task 5: Config Module

**Objective:** Create `src/config.rs` for user preferences (default author, default license, etc.).

**Files:**

- Create: `src/config.rs`
- Modify: `src/main.rs` (use config module)

**Step 1:** Create `src/config.rs`

```rust
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub default_author: Option<String>,
    pub default_license: Option<String>,
    pub year_override: Option<String>,
}

impl Config {
    /// Load config from XDG config home / licencify / config.toml
    pub fn load() -> Result<Self> {
        let path = config_path()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)
                .context("Failed to parse config file")?;
            Ok(config)
        } else {
            Ok(Config::default())
        }
    }

    /// Save config to XDG config home / licencify / config.toml
    pub fn save(&self) -> Result<()> {
        let path = config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Get effective author (config → git → fallback)
    pub fn effective_author(&self) -> Result<String> {
        if let Some(ref author) = self.default_author {
            return Ok(author.clone());
        }

        // Try git config
        let output = std::process::Command::new("git")
            .args(["config", "user.name"])
            .output()
            .ok();

        if let Some(out) = output {
            if out.status.success() {
                let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !name.is_empty() {
                    return Ok(name);
                }
            }
        }

        Err(anyhow::anyhow!(
            "No author name found. Set via: licencify config set author 'Your Name'"
        ))
    }
}

fn config_path() -> Result<PathBuf> {
    let base = dirs::config_dir()
        .context("Could not determine XDG config directory")?;
    Ok(base.join("licencify").join("config.toml"))
}
```

**Step 2:** Add `toml` dependency to `Cargo.toml`

```toml
toml = "0.8"
```

**Step 3:** Update `src/main.rs` to use `Config` for default values.

**Step 4:** Verify build
Run: `cargo check`
Expected: success.

**Step 5:** Commit

```bash
git add src/config.rs src/main.rs Cargo.toml
git commit "feat: add config module for user preferences"
```

---

## Task 6: Wire Everything Together

**Objective:** Update `src/main.rs` to integrate all new modules, implement the full 3-tier resolution chain, and ensure all commands work end-to-end.

**Files:**

- Modify: `src/main.rs`

**Step 1:** Update `cmd_add` to use:

1. Built-in templates (Tier 3) as first attempt
2. SPDX API fetch (Tier 2) as fallback
3. Cache (Tier 1) for subsequent requests

**Step 2:** Update `cmd_update` similarly.

**Step 3:** Update `cmd_detect` to use template module.

**Step 4:** Verify all commands work:

```bash
cargo run -- search mit
cargo run -- list --limit 10
cargo run -- add --help
cargo run -- cache info
```

**Step 5:** Commit

```bash
git add src/main.rs
git commit "feat: wire all modules together, implement 3-tier resolution chain"
```

---

## Task 7: Testing & Polish

**Objective:** Add integration tests and polish the CLI output.

**Files:**

- Create: `tests/integration.rs`
- Modify: `src/main.rs` (improve error messages)

**Step 1:** Create basic integration tests

```rust
#[test]
fn test_search_mit() {
    // Test that searching for "mit" returns results
}

#[test]
fn test_add_mit_offline() {
    // Test adding MIT license without network
}

#[test]
fn test_detect_no_license() {
    // Test detect in empty directory
}
```

**Step 2:** Add colored output for better UX (optional, using `colored` crate).

**Step 3:** Run tests

```bash
cargo test
```

**Step 4:** Commit

```bash
git add tests/ src/main.rs
git commit "test: add integration tests, polish CLI output"
```

---

## Summary of New Dependencies

```toml
[dependencies]
# Existing
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
anyhow = "1.0"
ureq = "3.0"
clap = { version = "4", features = ["derive"] }
dirs = "6.0"
chrono = "0.4"

# New
once_cell = "1"    # Task 1: lazy static for built-in templates
toml_edit = "0.22" # Task 4: Cargo.toml/pyproject.toml editing
toml = "0.8"       # Task 5: config file parsing
```

---

## Risks & Tradeoffs

| Risk                                            | Mitigation                                                         |
| ----------------------------------------------- | ------------------------------------------------------------------ |
| Built-in templates may drift from SPDX versions | Templates are static snapshots; `update-index` command can refresh |
| `toml_edit` may break complex TOML files        | Only modify `license` field, preserve rest via `DocumentMut`       |
| Config file format may change                   | Use TOML (human-readable, standard)                                |
| Network failures during SPDX fetch              | Built-in templates provide offline fallback                        |

---

## Open Questions

1. Should `licencify add` update manifests automatically by default, or require `--update-manifest` flag?
2. Should we add a `licencify config set/get` subcommand for managing preferences?
3. Should built-in templates include `[email]` placeholder support?
4. Should we add `--dry-run` flag to preview changes before applying?
