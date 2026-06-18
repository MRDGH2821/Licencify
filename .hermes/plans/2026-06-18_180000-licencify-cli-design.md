# Licencify — Rust CLI Tool Implementation Plan

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Build a Rust CLI tool (`licencify`) that adds open-source licenses to projects — fetching templates from GitHub's trusted License API, falling back to built-in templates, validating via SPDX identifiers, and optionally updating project manifest files.

**Architecture:** Single binary with clap CLI dispatch. Template resolution follows a 3-tier chain: (1) local disk cache, (2) GitHub Licenses REST API, (3) built-in embedded templates. Project file integration is opt-in per file type, using in-place editing for TOML files (preserving comments/formatting) and JSON rewrite for `package.json`. All SPDX validation uses the `spdx` crate.

**Tech Stack:** Rust (edition 2024), `clap` (derive), `spdx`, `serde`/`serde_json`, `toml_edit`, `ureq` (blocking HTTP), `dirs` (XDG paths), `anyhow`.

---

## Proposed Directory Structure

```
licencify/
├── Cargo.toml
├── src/
│   ├── main.rs              # Entry point, CLI dispatch
│   ├── cli.rs               # Clap CLI definition (subcommands + args)
│   ├── license.rs           # License resolution: remote → cache → built-in
│   ├── template.rs          # Template rendering (interpolate [year], [fullname])
│   ├── registry.rs          # Registry trait + GitHub API impl
│   ├── cache.rs             # XDG disk cache for fetched templates
│   ├── config.rs            # User config (authors, default license, registries)
│   ├── project/
│   │   ├── mod.rs           # Project detection + orchestration
│   │   ├── cargo.rs         # Cargo.toml read/write via toml_edit
│   │   ├── npm.rs           # package.json read/write via serde_json
│   │   └── python.rs        # pyproject.toml read/write via toml_edit
│   └── builtin/
│       ├── mod.rs           # Module exposing builtin::get(&str)
│       ├── mit.rs           # MIT text
│       ├── apache.rs        # Apache-2.0 text
│       ├── gpl3.rs          # GPL-3.0-only text
│       ├── gpl2.rs          # GPL-2.0-only text
│       ├── agpl3.rs         # AGPL-3.0-only text
│       ├── lgpl3.rs         # LGPL-3.0-only text
│       ├── bsd2.rs          # BSD-2-Clause text
│       ├── bsd3.rs          # BSD-3-Clause text
│       ├── mpl2.rs          # MPL-2.0 text
│       ├── unlicense.rs     # Unlicense text
│       └── cc01.rs          # CC0-1.0 text
```

---

## Task Plan

### Task 1: Scaffold the Rust project

**Objective:** Create Cargo.toml with all dependencies, set up workspace skeleton.

**Files:**

- Create: `Cargo.toml`
- Create: `src/main.rs` (minimal — just prints "hello")
- Modify: `.gitignore` (add `/target/`)

**Dependencies to add in Cargo.toml:**

```toml
[package]
name = "licencify"
version = "0.1.0"
edition = "2024"
description = "A CLI tool to add open-source licenses to your projects"
authors = ["MRDGH2821"]
license = "MIT"

[dependencies]
clap = { version = "4", features = ["derive"] }
spdx = "0.11"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml_edit = "0.22"
ureq = { version = "3", features = ["json"] }
dirs = "6"
anyhow = "1"
thiserror = "2"
colored = "3"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt"] }

[profile.release]
strip = true
lto = true
codegen-units = 1
```

**Step 1: Create Cargo.toml**
Write the above into `Cargo.toml`.

**Step 2: Create src/main.rs**

```rust
fn main() {
    println!("licencify — licenses, sorted.");
}
```

**Step 3: Update .gitignore**
Append `/target/` if not already present.

**Step 4: Verify build**
Run: `cargo check`
Expected: success, no warnings.

**Step 5: Commit**

```bash
git add Cargo.toml src/main.rs .gitignore
git commit -m "feat: scaffold Rust project with dependencies"
```

---

### Task 2: Define the CLI interface

**Objective:** Implement all subcommands with clap derive, ready for business logic.

**Files:**

- Create: `src/cli.rs`

**Command structure:**

```
licencify add <SPDX> [--author <name>] [--year <year>] [--yes]
licencify list [--remote]
licencify detect
licencify update <SPDX> [--author <name>] [--year <year>]
licencify cache clear
```

**Step 1: Write `src/cli.rs`**

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "licencify", about = "Add open-source licenses to your projects")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a license to the current project
    Add {
        /// SPDX license identifier (e.g., MIT, Apache-2.0, GPL-3.0-only)
        spdx: String,

        /// Copyright holder name (default: from config or git config user.name)
        #[arg(short, long)]
        author: Option<String>,

        /// Copyright year (default: current year)
        #[arg(short, long)]
        year: Option<String>,

        /// Skip all prompts and use defaults
        #[arg(short, long)]
        yes: bool,
    },

    /// List available licenses
    List {
        /// Also fetch remote license list from GitHub API
        #[arg(short, long)]
        remote: bool,
    },

    /// Detect the current project's license
    Detect,

    /// Update to a different license
    Update {
        /// SPDX license identifier to change to
        spdx: String,

        #[arg(short, long)]
        author: Option<String>,

        #[arg(short, long)]
        year: Option<String>,
    },

    /// Manage local template cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

#[derive(clap::Subcommand)]
pub enum CacheAction {
    /// Clear all cached templates
    Clear,
}
```

**Step 2: Wire CLI into `main.rs`**

```rust
mod cli;
use clap::Parser;

fn main() {
    let _cli = cli::Cli::parse();
    println!("licencify — licenses, sorted.");
}
```

**Step 3: Verify**
Run: `cargo check`
Expected: success.

**Step 4: Commit**

```bash
git add src/cli.rs src/main.rs Cargo.toml
git commit -m "feat: define CLI interface with clap subcommands"
```

---

### Task 3: Implement SPDX resolution + built-in templates

**Objective:** Validate SPDX identifiers using the `spdx` crate, and map them to built-in license texts.

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
- Create: `src/license.rs`

**Step 1: Create built-in template source files**
Each file contains the license text as a `&str`. Example (`src/builtin/mit.rs`):

```rust
pub const TEXT: &str = "MIT License

Copyright (c) [year] [fullname]

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the \"Software\"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
";
```

Create all 12 files with their respective SPDX texts. License texts sourced from [choosealicense.com](https://choosealicense.com/appendix/) / GitHub API.

**Step 2: Create `src/builtin/mod.rs`**

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

use std::collections::HashMap;
use std::sync::LazyLock;

/// Map of SPDX ID → license text (lowercased key)
static BUILTIN: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
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
    m
});

/// Look up a built-in template by lowercase SPDX ID.
/// Returns `None` if no built-in template exists for that ID.
pub fn get(spdx_lower: &str) -> Option<&'static str> {
    BUILTIN.get(spdx_lower).copied()
}

/// Return all supported SPDX IDs for built-in templates
pub fn supported_ids() -> Vec<&'static str> {
    let mut keys: Vec<&str> = BUILTIN.keys().copied().collect();
    keys.sort();
    keys
}
```

**Step 3: Create `src/license.rs` — SPDX resolution**

```rust
use spdx::Expression;

/// Validate an SPDX identifier string.
/// Returns the canonical SPDX expression if valid.
pub fn validate(spdx_str: &str) -> Result<String, String> {
    let expr = Expression::parse(spdx_str)
        .map_err(|e| format!("Invalid SPDX expression '{spdx_str}': {e}"))?;

    // Extract the primary license ID (assumes single-license, simplest case)
    let id = expr
        .license_id()
        .ok_or_else(|| "Only single-license expressions are supported".to_string())?;

    Ok(id.to_owned())
}

/// Normalize an SPDX ID to lowercase for lookups
pub fn normalize(spdx_id: &str) -> String {
    spdx_id.to_lowercase()
}

/// The GitHub API uses different keys than SPDX IDs.
/// Map SPDX ID → GitHub API key for license fetching.
pub fn to_github_key(spdx_id: &str) -> String {
    match spdx_id {
        "AGPL-3.0-only" | "AGPL-3.0-or-later" => "agpl-3.0".to_string(),
        "GPL-3.0-only" | "GPL-3.0-or-later" => "gpl-3.0".to_string(),
        "GPL-2.0-only" | "GPL-2.0-or-later" => "gpl-2.0".to_string(),
        "LGPL-3.0-only" | "LGPL-3.0-or-later" => "lgpl-3.0".to_string(),
        "LGPL-2.1-only" | "LGPL-2.1-or-later" => "lgpl-2.1".to_string(),
        other => other.to_lowercase(),
    }
}
```

**Step 4: Verify build**
Run: `cargo check`
Expected: success, no warnings.

**Step 5: Commit**

```bash
git add src/builtin/ src/license.rs Cargo.toml
git commit -m "feat: SPDX validation and built-in license templates"
```

---

### Task 4: Template rendering engine

**Objective:** Replace `[year]` and `[fullname]` placeholders with user-supplied values.

**Files:**

- Create: `src/template.rs`

**Step 1: Write `src/template.rs`**

```rust
use anyhow::{Context, Result};

/// Rendered license with metadata
pub struct RenderedLicense {
    /// The complete license text with placeholders replaced
    pub body: String,
    /// Which variables were filled
    pub year: String,
    pub fullname: String,
}

/// Render a license template by replacing placeholders with actual values.
///
/// Recognized placeholders (from GitHub's license API format):
/// - `[year]` → the current year or user-supplied year
/// - `[fullname]` → the author/copyright holder name
/// - `[yyyy]` → same as [year] (alternative format)
/// - `[name of copyright owner]` → same as [fullname] (Apache-2.0 uses this)
/// - `[copyright holder]` → same as [fullname] (BSD uses this)
pub fn render(
    template: &str,
    year: &str,
    fullname: &str,
) -> RenderedLicense {
    let body = template
        .replace("[year]", year)
        .replace("[yyyy]", year)
        .replace("[fullname]", fullname)
        .replace("[name of copyright owner]", fullname)
        .replace("[copyright holder]", fullname)
        .replace("[copyright holder(s)]", fullname);

    RenderedLicense {
        body,
        year: year.to_string(),
        fullname: fullname.to_string(),
    }
}

/// Resolve the current year (user-supplied or current)
pub fn resolve_year(user_year: Option<String>) -> String {
    user_year.unwrap_or_else(|| {
        // Use chrono or just let the OS provide it
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();
        // Approximate: 1970 + seconds/31557600
        let years_since_epoch = now.as_secs() as f64 / 31_557_600.0;
        let year = 1970 + years_since_epoch as u64;
        year.to_string()
    })
}

/// Resolve the author name (user-supplied or from git config)
pub fn resolve_author(user_author: Option<String>) -> Result<String> {
    if let Some(name) = user_author {
        return Ok(name);
    }

    // Try reading from git config
    let output = std::process::Command::new("git")
        .args(["config", "user.name"])
        .output()
        .context("Could not determine author name. Use --author or set git user.name")?;

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
```

**Step 2: Verify build**
Run: `cargo check`
Expected: success.

**Step 3: Commit**

```bash
git add src/template.rs Cargo.toml
git commit -m "feat: template rendering with placeholder interpolation"
```

---

### Task 5: Remote registry + caching

**Objective:** Fetch license templates from GitHub's Licenses API, cache to XDG cache dir, implement fallback chain.

**Files:**

- Create: `src/cache.rs`
- Create: `src/registry.rs`

**Step 1: Create `src/cache.rs`**

```rust
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Manages a local on-disk cache of fetched license templates.
pub struct LicenseCache {
    cache_dir: PathBuf,
}

impl LicenseCache {
    /// Create cache in XDG cache home / licencify / templates
    pub fn new() -> Result<Self> {
        let base = dirs::cache_dir()
            .context("Could not determine XDG cache directory")?;
        let cache_dir = base.join("licencify").join("templates");
        Ok(Self { cache_dir })
    }

    /// Path where a license with the given lowercased SPDX key would be cached
    pub fn path_for(&self, spdx_key: &str) -> PathBuf {
        self.cache_dir.join(format!("{spdx_key}.json"))
    }

    /// Try to read a cached license. Returns `None` if not in cache.
    pub fn get(&self, spdx_key: &str) -> Option<String> {
        let path = self.path_for(spdx_key);
        if path.exists() {
            std::fs::read_to_string(&path).ok()
        } else {
            None
        }
    }

    /// Store a downloaded license body into the cache.
    pub fn put(&self, spdx_key: &str, body: &str) -> Result<()> {
        std::fs::create_dir_all(&self.cache_dir)
            .context("Failed to create cache directory")?;
        std::fs::write(self.path_for(spdx_key), body)
            .context("Failed to write cache file")
    }

    /// Clear all cached templates
    pub fn clear(&self) -> Result<()> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)
                .context("Failed to clear cache")?;
        }
        Ok(())
    }
}
```

**Step 2: Create `src/registry.rs`**

```rust
use crate::cache::LicenseCache;
use crate::license;
use anyhow::{Context, Result};
use serde::Deserialize;

/// Response from GitHub's License API
#[derive(Deserialize)]
struct GithubLicenseResponse {
    body: String,
}

/// Resolution chain result
pub enum LicenseSource {
    /// Fetched from GitHub API
    Remote(String),
    /// Loaded from disk cache
    Cached(String),
    /// From built-in templates
    BuiltIn(String),
}

/// Resolve a license template following the chain:
/// 1. Disk cache (fastest)
/// 2. GitHub Licenses API
/// 3. Built-in embedded templates (always works, never empty)
pub fn resolve(spdx_id: &str, cache: &LicenseCache) -> Result<(String, LicenseSource)> {
    let lower = license::normalize(spdx_id);
    let github_key = license::to_github_key(spdx_id);

    // 1. Check cache
    if let Some(body) = cache.get(&lower) {
        return Ok((body, LicenseSource::Cached(lower)));
    }

    // 2. Try GitHub API
    match fetch_from_github(&github_key) {
        Ok(body) => {
            // Don't fail the whole operation if caching fails
            let _ = cache.put(&lower, &body);
            return Ok((body, LicenseSource::Remote(github_key)));
        }
        Err(e) => {
            eprintln!("  ⚠ GitHub API unavailable: {e}");
            eprintln!("  ⚠ Falling back to built-in template");
        }
    }

    // 3. Built-in fallback
    match crate::builtin::get(&lower) {
        Some(text) => Ok((text.to_string(), LicenseSource::BuiltIn(lower))),
        None => anyhow::bail!(
            "License '{spdx_id}' is not available as a built-in template and \
             could not be fetched remotely. Try `licencify list --remote` to \
             see available remote licenses, or use one of the built-in ones: {}",
            crate::builtin::supported_ids().join(", ")
        ),
    }
}

/// Fetch license text from GitHub's Licenses API.
/// Uses the GitHub API key (lowercase, hyphenated), not the SPDX ID.
fn fetch_from_github(github_key: &str) -> Result<String> {
    let url = format!("https://api.github.com/licenses/{github_key}");
    let response: GithubLicenseResponse = ureq::get(&url)
        .set("User-Agent", "licencify/0.1.0")
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .context("Failed to contact GitHub API")?
        .into_body()
        .read_json()
        .context("Failed to parse GitHub API response")?;

    Ok(response.body)
}

/// List available licenses from GitHub
pub fn list_remote() -> Result<Vec<String>> {
    #[derive(Deserialize)]
    struct LicenseSummary {
        spdx_id: String,
    }

    let licenses: Vec<LicenseSummary> = ureq::get("https://api.github.com/licenses")
        .set("User-Agent", "licencify/0.1.0")
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .context("Failed to fetch license list from GitHub")?
        .into_body()
        .read_json()
        .context("Failed to parse license list")?;

    Ok(licenses.into_iter().map(|l| l.spdx_id).collect())
}
```

**Step 3: Verify build**
Run: `cargo check`
Expected: success.

**Step 4: Commit**

```bash
git add src/cache.rs src/registry.rs Cargo.toml
git commit -m "feat: GitHub API registry and XDG disk cache"
```

---

### Task 6: Project file detection and integration

**Objective:** Detect project type by scanning for manifest files, read/write license fields in Cargo.toml, package.json, pyproject.toml.

**Files:**

- Create: `src/project/mod.rs`
- Create: `src/project/cargo.rs`
- Create: `src/project/npm.rs`
- Create: `src/project/python.rs`

**Step 1: Create `src/project/cargo.rs`**

```rust
use anyhow::{Context, Result};
use std::path::Path;
use toml_edit::{DocumentMut, value};

/// Read the existing license field from a Cargo.toml if present
pub fn detect_license(path: &Path) -> Result<Option<String>> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read Cargo.toml")?;
    let doc = content.parse[IP_REDACTED]<DocumentMut>()
        .context("Failed to parse Cargo.toml")?;

    Ok(doc["package"]["license"]
        .as_str()
        .map(|s| s.to_string()))
}

/// Set (or update) the license field in Cargo.toml.
/// Uses toml_edit to preserve formatting and comments.
pub fn set_license(path: &Path, spdx: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read Cargo.toml")?;
    let mut doc = content.parse::<DocumentMut>()
        .context("Failed to parse Cargo.toml")?;

    doc["package"]["license"] = value(spdx);

    std::fs::write(path, doc.to_string())
        .context("Failed to write Cargo.toml")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_license_in_simple_toml() {
        let dir = std::env::temp_dir().join("licencify-test-cargo");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Cargo.toml");
        std::fs::write(&path, r#"[package]
name = "test"
version = "0.1.0"
edition = "2024"
"#).unwrap();

        set_license(&path, "MIT").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("license = \"MIT\""));
    }
}
```

**Step 2: Create `src/project/npm.rs`**

```rust
use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::path::Path;

/// Read the existing license field from package.json
pub fn detect_license(path: &Path) -> Result<Option<String>> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read package.json")?;
    let json: Value = serde_json::from_str(&content)
        .context("Failed to parse package.json")?;

    Ok(json.get("license").and_then(|v| v.as_str()).map(|s| s.to_string()))
}

/// Set the license field in package.json.
/// Preserves all existing fields and formatting (serde_json re-serializes).
pub fn set_license(path: &Path, spdx: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read package.json")?;
    let mut json: Value = serde_json::from_str(&content)
        .context("Failed to parse package.json")?;

    if let Value::Object(ref mut map) = json {
        map.insert("license".to_string(), Value::String(spdx.to_string()));
    }

    let out = serde_json::to_string_pretty(&json)
        .context("Failed to serialize package.json")?;
    std::fs::write(path, out)
        .context("Failed to write package.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_license_in_package_json() {
        let dir = std::env::temp_dir().join("licencify-test-npm");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("package.json");
        std::fs::write(&path, r#"{"name":"test","version":"1.0.0"}"#).unwrap();

        set_license(&path, "MIT").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("\"license\": \"MIT\""));
    }
}
```

**Step 3: Create `src/project/python.rs`**

```rust
use anyhow::{Context, Result};
use std::path::Path;
use toml_edit[IP_REDACTED]{DocumentMut, value};

/// Read license from pyproject.toml (located at [project.license] or [tool.poetry.license])
pub fn detect_license(path: &Path) -> Result<Option<String>> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read pyproject.toml")?;
    let doc = content.pars[IP_REDACTED]<DocumentMut>()
        .context("Failed to parse pyproject.toml")?;

    // PEP 621: [project.license] can be a string or table
    if let Some(val) = doc.get("project").and_then(|p| p.get("license")) {
        if let Some(s) = val.as_str() {
            return Ok(Some(s.to_string()));
        }
        if let Some(text) = val.get("text").and_then(|t| t.as_str()) {
            return Ok(Some(text.to_string()));
        }
    }

    // Poetry format: [tool.poetry.license]
    if let Some(val) = doc.get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("license"))
        .and_then(|l| l.as_str())
    {
        return Ok(Some(val.to_string()));
    }

    Ok(None)
}

/// Set the license in pyproject.toml (PEP 621 style)
pub fn set_license(path: &Path, spdx: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read pyproject.toml")?;
    let mut doc = content.pars[IP_REDACTED]<DocumentMut>()
        .context("Failed to parse pyproject.toml")?;

    // Prefer PEP 621
    if doc.contains_table("project") {
        doc["project"]["license"] = value(spdx);
    } else if doc.contains_table("tool") && doc["tool"].contains_table("poetry") {
        doc["tool"]["poetry"]["license"] = value(spdx);
    } else {
        // Add project table
        doc["project"]["license"] = value(spdx);
    }

    std::fs::write(path, doc.to_string())
        .context("Failed to write pyproject.toml")
}
```

**Step 4: Create `src/project/mod.rs`**

```rust
mod cargo;
mod npm;
mod python;

use anyhow::Result;
use std::path::{Path, PathBuf};

/// A detected project manifest file
pub enum Manifest {
    CargoToml(PathBuf),
    PackageJson(PathBuf),
    PyProjectToml(PathBuf),
}

impl Manifest {
    pub fn path(&self) -> &Path {
        match self {
            Manifest::CargoToml(p) => p,
            Manifest::PackageJson(p) => p,
            Manifest::PyProjectToml(p) => p,
        }
    }

    /// Detect existing license in this manifest
    pub fn detect_license(&self) -> Result<Option<String>> {
        match self {
            Manifest::CargoToml(p) => cargo::detect_license(p),
            Manifest::PackageJson(p) => npm::detect_license(p),
            Manifest::PyProjectToml(p) => python::detect_license(p),
        }
    }

    /// Set the license in this manifest
    pub fn set_license(&self, spdx: &str) -> Result<()> {
        match self {
            Manifest::CargoToml(p) => cargo::set_license(p, spdx),
            Manifest::PackageJson(p) => npm::set_license(p, spdx),
            Manifest::PyProjectToml(p) => python::set_license(p, spdx),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Manifest::CargoToml(_) => "Cargo.toml",
            Manifest::PackageJson(_) => "package.json",
            Manifest::PyProjectToml(_) => "pyproject.toml",
        }
    }
}

/// Scan a directory for supported project manifest files
pub fn detect_manifests(dir: &Path) -> Vec<Manifest> {
    let mut manifests = Vec::new();

    let candidates = [
        ("Cargo.toml", Manifest::CargoToml as fn(PathBuf) -> Manifest),
        ("package.json", Manifest::PackageJson),
        ("pyproject.toml", Manifest::PyProjectToml),
    ];

    for (filename, ctor) in &candidates {
        let path = dir.join(filename);
        if path.exists() {
            manifests.push(ctor(path));
        }
    }

    manifests
}
```

**Step 5: Run tests**
Run: `cargo test`
Expected: cargo and npm tests pass.

**Step 6: Commit**

```bash
git add src/project/
git commit -m "feat: project file detection (Cargo.toml, package.json, pyproject.toml)"
```

---

### Task 7: Wire everything together in main

**Objective:** Implement the full `add`, `list`, `detect`, `update`, `cache clear` commands.

**Files:**

- Modify: `src/main.rs`

**Step 1: Replace `src/main.rs` with full implementation**

```rust
mod builtin;
mod cache;
mod cli;
mod license;
mod project;
mod registry;
mod template;

use anyhow::{Context, Result};
use clap::Parser;
use colored::*;

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let cli = cli::Cli::parse();

    match cli.command {
        cli::Commands::Add { spdx, author, year, yes } => {
            cmd_add(&spdx, author, year, yes)?;
        }
        cli::Commands::List { remote } => {
            cmd_list(remote)?;
        }
        cli::Commands::Detect => {
            cmd_detect()?;
        }
        cli::Commands::Update { spdx, author, year } => {
            cmd_add(&spdx, author, year, true)?;
        }
        cli::Commands::Cache { action } => {
            match action {
                cli::CacheAction::Clear => {
                    let cache = cache::LicenseCache::new()?;
                    cache.clear()?;
                    println!("{}", "✓ Cache cleared".green());
                }
            }
        }
    }

    Ok(())
}

fn cmd_add(spdx_str: &str, author: Option<String>, year: Option<String>, _yes: bool) -> Result<()> {
    // 1. Validate SPDX
    let canonical = license::validate(spdx_str)
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    println!("  ℹ Using license: {}", canonical.green());

    // 2. Resolve template
    let cache = cache::LicenseCache::new()?;
    let (template_body, source) = registry::resolve(&canonical, &cache)?;
    match source {
        registry::LicenseSource::Remote(k) => println!("  ℹ Fetched from GitHub API ({})", k),
        registry::LicenseSource::Cached(_) => println!("  ℹ Loaded from cache"),
        registry::LicenseSource::BuiltIn(_) => println!("  ℹ Using built-in template"),
    }

    // 3. Resolve author & year
    let year = template::resolve_year(year);
    let fullname = template::resolve_author(author)?;

    // 4. Render template
    let rendered = template::render(&template_body, &year, &fullname);

    // 5. Write LICENSE file
    let license_path = std::env::current_dir()?.join("LICENSE");
    // Check if LICENSE already exists
    if license_path.exists() {
        eprintln!("  ⚠ LICENSE already exists. Overwrite? [y/N] ");
        // In non-interactive mode, still write
    }
    std::fs::write(&license_path, &rendered.body)
        .context("Failed to write LICENSE file")?;
    println!("  ✓ Written LICENSE for {} ({})", rendered.fullname, rendered.year);

    // 6. Detect project manifests and offer to update
    let manifests = project::detect_manifests(&std::env::current_dir()?);
    for manifest in &manifests {
        match manifest.detect_license() {
            Ok(Some(current)) => {
                println!("  ℹ {} has license: {}", manifest.name(), current);
                // Update it
                if let Err(e) = manifest.set_license(&canonical) {
                    eprintln!("  ⚠ Failed to update {}: {e}", manifest.name());
                } else {
                    println!("  ✓ Updated {}", manifest.name());
                }
            }
            Ok(None) => {
                println!("  ℹ {} has no license field — adding", manifest.name());
                if let Err(e) = manifest.set_license(&canonical) {
                    eprintln!("  ⚠ Failed to update {}: {e}", manifest.name());
                } else {
                    println!("  ✓ Updated {}", manifest.name());
                }
            }
            Err(e) => {
                eprintln!("  ⚠ Could not read {}: {e}", manifest.name());
            }
        }
    }

    Ok(())
}

fn cmd_list(remote: bool) -> Result<()> {
    println!("{}", "Built-in licenses:".green());
    for id in builtin::supported_ids() {
        println!("  • {}", id);
    }

    if remote {
        println!("\n{}", "Remote (GitHub API) licenses:".cyan());
        match registry::list_remote() {
            Ok(ids) => {
                for id in ids {
                    println!("  • {}", id);
                }
            }
            Err(e) => {
                eprintln!("  ⚠ Could not fetch remote list: {e}");
            }
        }
    }

    Ok(())
}

fn cmd_detect() -> Result<()> {
    let cwd = std::env::current_dir()?;

    // Check for LICENSE file
    for name in &["LICENSE", "LICENSE.txt", "LICENSE.md", "LICENCE"] {
        let path = cwd.join(name);
        if path.exists() {
            println!("  ✓ Found license file: {}", name);
            // Read first line as hint
            if let Ok(content) = std::fs::read_to_string(&path) {
                let first_line = content.lines().next().unwrap_or("");
                println!("  ℹ First line: {}", first_line);
            }
        }
    }

    // Check manifests
    let manifests = project::detect_manifests(&cwd);
    for manifest in &manifests {
        match manifest.detect_license() {
            Ok(Some(lic)) => println!("  ✓ {} → {}", manifest.name(), lic),
            Ok(None) => println!("  ℹ {} → no license field", manifest.name()),
            Err(e) => eprintln!("  ⚠ {} → error: {e}", manifest.name()),
        }
    }

    Ok(())
}
```

**Step 2: Verify build**
Run: `cargo check`
Expected: success (a few warnings about unused imports/variables is ok).

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire up all commands in main entry point"
```

---

### Task 8: Config module

**Objective:** Persistent user configuration for default author, default license, custom registries.

**Files:**

- Create: `src/config.rs`

**Step 1: Write `src/config.rs`**

```rust
use anyhow::{Context, Result};
use serd[IP_REDACTED]{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Default copyright holder name
    pub default_author: Option<String>,
    /// Default license to use when not specified
    pub default_license: Option<String>,
    /// Custom template registries (URLs that return license JSON)
    pub registries: Vec<RegistryConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub name: String,
    pub url: String,
    pub priority: u32, // lower = higher priority
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_author: None,
            default_license: None,
            registries: vec![RegistryConfig {
                name: "github".to_string(),
                url: "https://api.github.com/licenses".to_string(),
                priority: 10,
            }],
        }
    }
}

impl Config {
    /// XDG config path for licencify
    pub fn path() -> Result<PathBuf> {
        let base = dirs::config_dir()
            .context("Could not find XDG config directory")?;
        Ok(base.join("licencify").join("config.toml"))
    }

    /// Load config from disk, or return defaults if it doesn't exist
    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .context("Failed to read config")?;
            toml_edit::de::from_str(&content)
                .context("Failed to parse config file")
        } else {
            Ok(Config::default())
        }
    }

    /// Save config to disk
    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        std::fs::create_dir_all(path.parent().unwrap())
            .context("Failed to create config directory")?;
        let content = toml_edit::ser::to_string_pretty(self)
            .context("Failed to serialize config")?;
        std::fs::write(&path, content)
            .context("Failed to write config file")
    }
}
```

**Step 2: Wire config into main**

- In `cmd_add`, try `Config::load()` for default_author and default_license hints.
- Only create config file if user runs `licencify config init` (add CLI subcommand).

**Step 3: Verify build**
Run: `cargo check`
Expected: success.

**Step 4: Commit**

```bash
git add src/config.rs Cargo.toml
git commit -m "feat: persistent config (XDG-based, TOML)"
```

---

### Task 9: Polish, error messages, and final integration

**Objective:** Colored output, meaningful error messages, edge cases, cleanup.

**Files:**

- Modify: `src/main.rs`

**Step 1:** Add `init_subcommand` for config init.

**Step 2:** Add `--quiet` / `-q` flag to suppress non-essential output.

**Step 3:** Handle edge cases:

- No LICENSE file already exists → skip overwrite prompt in non-interactive
- No network → graceful built-in fallback (already handled)
- Invalid SPDX → clear error suggesting `licencify list`
- Empty author → error with suggestion

**Step 4: Verify full build**
Run: `cargo build --release`
Expected: success, binary at `target/release/licencify`.

**Step 5: Manual smoke test**

```bash
# Run without args
./target/debug/licencify --help

# List built-in licenses
./target/debug/licencify list

# Try adding MIT
./target/debug/licencify add MIT
```

**Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat: polish, error handling, final integration"
```

---

## Validation Plan

| Test                   | What to Verify                         | Command                                                             |
| ---------------------- | -------------------------------------- | ------------------------------------------------------------------- |
| Build                  | Compiles without errors                | `cargo build --release`                                             |
| Tests                  | Unit tests pass                        | `cargo test`                                                        |
| CLI help               | All subcommands show                   | `./target/debug/licencify --help`                                   |
| List builtins          | Shows 15+ licenses                     | `./target/debug/licencify list`                                     |
| Add MIT                | Creates LICENSE with rendered template | `./target/debug/licencify add MIT --author "Test User" --year 2026` |
| Add invalid SPDX       | Clear error message                    | `./target/debug/licencify add INVALID`                              |
| Detect                 | Shows existing license info            | `./target/debug/licencify detect`                                   |
| Cargo.toml integration | Sets license field                     | Run in project with Cargo.toml                                      |
| Cache clear            | Clears templates                       | `./target/debug/licencify cache clear`                              |

---

## Risks & Tradeoffs

| Risk                                                        | Mitigation                                                                                                                             |
| ----------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| **GitHub API rate limiting** (60 req/hr unauthenticated)    | Aggressive caching; cache-first lookup; user can set GH token in config                                                                |
| **Binary size from embedded templates**                     | Template text files are small (~1-2KB each); ~15 templates ≈ 30KB total — negligible                                                   |
| **`spdx` crate license text may differ from GitHub's text** | Slight wording differences are acceptable since both are official; GitHub API is authoritative source, built-in templates are fallback |
| **`ureq` is blocking**                                      | Acceptable for a CLI tool; no need for async runtime                                                                                   |
| **`edition = "2024"` may be unstable**                      | Use `edition = "2021"` if 2024 causes issues on stable Rust                                                                            |
| **pyproject.toml has multiple formats**                     | Support PEP 621 `[project.license]` and Poetry `[tool.poetry.license]`                                                                 |

---

## Open Questions

1. **Interactive prompts** — Should `add` ask before overwriting an existing LICENSE? (Current plan: warn but overwrite when `--yes` is set)
2. **Header-only mode** — Some users may want to add SPDX license headers to source files (not just root LICENSE). Post-MVP.
3. **Custom registries** — Allow users to point to self-hosted license template repos via config. Config module supports this, but no CLI subcommand yet.
4. **Dual-license** — SPDX expression like `MIT OR Apache-2.0` — skip for MVP, document limitation.
5. **GPL `-only` vs `-or-later`** — Built-in templates use the same text for both variants (they share identical body text, only the SPDX identifier differs). This matches how GitHub's API treats them.

---

## Execution Handoff

**\*Plan complete and saved. Ready to execute using subagent-driven-development — I'll dispatch a fresh subagent per task with two-stage review (spec compliance then code quality). Shall I proceed?**
