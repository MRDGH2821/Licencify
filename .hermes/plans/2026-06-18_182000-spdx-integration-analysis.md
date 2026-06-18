# SPDX Licenses JSON — Integration Analysis & Plan

> **For Hermes:** This plan upgrades the existing licencify CLI design to use SPDX as the primary license data source instead of (or alongside) the GitHub Licenses API.

**Goal:** Replace the limited GitHub Licenses API (13 licenses) with the SPDX registry (727 licenses) as the authoritative license source, keeping the 3-tier resolution chain but with vastly broader coverage.

**Architecture:** SPDX provides two data layers: a flat registry index (`licenses.json`, 332KB) for validation/discovery, and per-license detail files (`{licenseId}.json`) containing full license text + machine-readable templates. Both are S3-hosted, no auth required, CDN-cached.

**Tech Stack:** Same as existing plan — Rust, `clap`, `serde_json`, `ureq`, `dirs`, `anyhow`.

---

## Verdict: Highly Useful ✅

| Dimension        | GitHub `/licenses`             | SPDX `licenses.json`                                              |
| ---------------- | ------------------------------ | ----------------------------------------------------------------- |
| Coverage         | 13 licenses                    | **727 licenses** (56×)                                            |
| Auth required    | Yes (token for raw text)       | **No**                                                            |
| Template text    | Raw text via separate endpoint | **`licenseText` + `standardLicenseTemplate`** in detail JSON      |
| Metadata         | `spdx_id`, `name`, `html_url`  | `isOsiApproved`, `isFsfLibre`, `isDeprecatedLicenseId`, `seeAlso` |
| Versioning       | None                           | **Versioned** (currently 3.28.0, updated 2026-02-20)              |
| Reliability      | GitHub API rate limits         | **S3/CloudFront**, no rate limits                                 |
| Popular licenses | 13/14 found                    | **14/14 found** + 713 more                                        |

**Key finding:** SPDX has every popular license GitHub offers, plus 713 more, with richer metadata and no auth. The only GitHub advantage is its `standardLicenseTemplate` uses `<<var;name=...>>` syntax which needs parsing — but `licenseText` with simple `<year>` / `<copyright holders>` placeholders is sufficient for licencify's needs.

---

## SPDX Data Structure

### Registry Index (`licenses.json`)

```
GET https://spdx.org/licenses/licenses.json
332KB | No auth | CDN-cached
```

Fields per entry:

```json
{
  "licenseId": "MIT",
  "name": "MIT License",
  "reference": "https://spdx.org/licenses/MIT.html",
  "isDeprecatedLicenseId": false,
  "isOsiApproved": true,
  "isFsfLibre": true,
  "detailsUrl": "https://spdx.org/licenses/MIT.json",
  "seeAlso": ["https://opensource.org/licenses/MIT"],
  "referenceNumber": 498
}
```

### Per-License Detail (`{licenseId}.json`)

```
GET https://spdx.org/licenses/MIT.json
~2-5KB per license | No auth
```

Fields:

```json
{
  "licenseId": "MIT",
  "name": "MIT License",
  "licenseText": "MIT License\n\nCopyright (c) <year> <copyright holders>\n\nPermission is hereby granted...",
  "standardLicenseTemplate": "<<beginOptional>>MIT License<<endOptional>> <<var;name=\"copyright\";...>>",
  "licenseTextHtml": "<div class=\"...\">...",
  "isDeprecatedLicenseId": false,
  "isOsiApproved": true,
  "isFsfLibre": true
}
```

**Placeholder convention:** `<year>` and `<copyright holders>` are the standard SPDX placeholders. Simple string replace works for most licenses.

---

## Updated Architecture: SPDX-First Resolution

### Revised 3-Tier Chain

```
Tier 1: Local XDG disk cache (fastest, $0)
Tier 2: SPDX detail URL (727 licenses, no auth, CDN)
Tier 3: Built-in embedded templates (MIT, Apache-2.0, GPL-3.0, BSD-3 — always works)
```

GitHub API demoted to **Tier 2b** (optional, behind feature flag, for users who prefer GitHub's raw text formatting).

### Validation: SPDX Index as Source of Truth

Instead of the `spdx` crate for validation, use the SPDX index directly:

1. **Embed `licenses.json` at compile time** via `include_str!("data/licenses.json")` or `include_bytes!` — adds ~332KB to binary, but means license validation works fully offline with zero network.
2. **Build a `HashMap<LicenseId, LicenseEntry>` at runtime** for O(1) lookup.
3. **Expose metadata** (`isOsiApproved`, `isFsfLibre`, `isDeprecated`) in CLI output — this is free value GitHub doesn't provide.

### Caching Strategy

- **Index:** Embedded in binary, re-fetched periodically (e.g., `licencify update-index`). Store in XDG cache dir.
- **Templates:** Fetched on demand, cached in XDG cache dir under `templates/{licenseId}.json`. Never re-fetched once cached.

---

## Files Changed vs Original Plan

| Original Plan        | Change                                     | Reason                                    |
| -------------------- | ------------------------------------------ | ----------------------------------------- |
| `src/registry.rs`    | **Rewrite** — SPDX registry impl           | SPDX is primary source, not GitHub        |
| `src/license.rs`     | **Modify** — use SPDX index for validation | Replace `spdx` crate with embedded index  |
| `src/cache.rs`       | **Modify** — cache SPDX detail JSONs       | Cache `licenseText` from detail files     |
| `src/builtin/mod.rs` | **Keep** — still Tier 3 fallback           | Unchanged                                 |
| `Cargo.toml`         | **Remove** `spdx` crate                    | No longer needed — SPDX index replaces it |
| `src/cli.rs`         | **Add** — `licencify search` subcommand    | Free: search 727 licenses by name/keyword |
| `data/licenses.json` | **Create** — bundled SPDX index            | Embedded at compile time                  |
| NEW: `src/spdx.rs`   | **Create** — SPDX types + parser           | Serde types for registry + detail JSON    |

---

## Implementation Tasks

### Task 1: Download and embed SPDX index

**Objective:** Fetch `licenses.json` and embed it in the binary.

**Files:**

- Create: `data/licenses.json` (downloaded)
- Create: `src/spdx.rs` (types + parser)
- Modify: `src/main.rs` (add `mod spdx`)

**Step 1:** Download the SPDX index

```bash
curl -s https://spdx.org/licenses/licenses.json -o data/licenses.json
```

**Step 2:** Create SPDX types in `src/spdx.rs`

```rust
use serde::Deserialize;
use std::collections::HashMap;

const SPDX_INDEX: &str = include_str!("../data/licenses.json");

#[derive(Debug, Deserialize)]
pub struct SpdxIndex {
    pub license_list_version: String,
    pub release_date: String,
    pub licenses: Vec<SpdxLicense>,
}

#[derive(Debug, Deserialize)]
pub struct SpdxLicense {
    pub license_id: String,
    pub name: String,
    #[serde(default)]
    pub is_deprecated_license_id: bool,
    #[serde(default)]
    pub is_osi_approved: bool,
    #[serde(default)]
    pub is_fsf_libre: bool,
    pub details_url: Option<String>,
    #[serde(default)]
    pub see_also: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SpdxLicenseDetail {
    pub license_id: String,
    pub name: String,
    pub license_text: String,
    #[serde(default)]
    pub is_osi_approved: bool,
    #[serde(default)]
    pub is_fsf_libre: bool,
}

impl SpdxIndex {
    pub fn load() -> anyhow::Result<Self> {
        let index: SpdxIndex = serde_json::from_str(SPDX_INDEX)?;
        Ok(index)
    }

    pub fn find(&self, license_id: &str) -> Option<&SpdxLicense> {
        self.licenses.iter().find(|l| l.license_id == license_id)
    }

    pub fn search(&self, query: &str) -> Vec<&SpdxLicense> {
        let q = query.to_lowercase();
        self.licenses.iter().filter(|l| {
            l.name.to_lowercase().contains(&q) || l.license_id.to_lowercase().contains(&q)
        }).collect()
    }

    pub fn by_id(&self) -> HashMap<&str, &SpdxLicense> {
        self.licenses.iter().map(|l| (l.license_id.as_str(), l)).collect()
    }
}
```

**Step 3:** Add to `src/main.rs`

```rust
mod spdx;
```

**Step 4:** Verify it compiles

```bash
cargo build 2>&1
```

**Step 5:** Commit

```bash
git add data/licenses.json src/spdx.rs src/main.rs
git commit -m "feat: embed SPDX license index (727 licenses)"
```

---

### Task 2: Replace `spdx` crate validation with SPDX index

**Objective:** Use the embedded SPDX index for license ID validation instead of the `spdx` crate.

**Files:**

- Modify: `src/license.rs` (validation logic)
- Modify: `Cargo.toml` (remove `spdx` if present)

**Step 1:** Remove `spdx` from `Cargo.toml` dependencies (currently empty anyway)

**Step 2:** Add validation to `src/spdx.rs`

```rust
impl SpdxIndex {
    pub fn is_valid(&self, license_id: &str) -> bool {
        self.find(license_id).is_some()
    }

    pub fn is_deprecated(&self, license_id: &str) -> bool {
        self.find(license_id)
            .map(|l| l.is_deprecated_license_id)
            .unwrap_or(false)
    }
}
```

**Step 3:** Commit

```bash
git add src/spdx.rs Cargo.toml
git commit -m "feat: SPDX index-based validation (replaces spdx crate)"
```

---

### Task 3: SPDX detail fetcher (replaces GitHub API)

**Objective:** Fetch license templates from SPDX detail URLs instead of GitHub API.

**Files:**

- Modify: `src/registry.rs` (rewrite as SPDX fetcher)
- Modify: `src/spdx.rs` (add `fetch_detail`)

**Step 1:** Implement SPDX detail fetching in `src/spdx.rs`

```rust
use std::fs;

impl SpdxIndex {
    pub fn detail_url(&self, license_id: &str) -> Option<String> {
        self.find(license_id)
            .and_then(|l| l.details_url.clone())
    }

    pub fn fetch_detail(license_id: &str, cache_dir: &std::path::Path) -> anyhow::Result<SpdxLicenseDetail> {
        let cache_path = cache_dir.join(format!("{license_id}.json"));

        // Check cache first
        if cache_path.exists() {
            let text = fs::read_to_string(&cache_path)?;
            return Ok(serde_json::from_str(&text)?);
        }

        // Build URL from convention
        let url = format!("https://spdx.org/licenses/{license_id}.json");
        let body = ureq::get(&url)
            .call()?
            .into_reader()
            .read_to_string()?;

        // Cache it
        if let Some(parent) = cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&cache_path, &body)?;

        Ok(serde_json::from_str(&body)?)
    }
}
```

**Step 2:** Rewrite `src/registry.rs` as SPDX registry

```rust
use crate::spdx::{SpdxIndex, SpdxLicenseDetail};
use anyhow::{Context, Result};
use std::path::Path;

pub struct Registry {
    index: SpdxIndex,
    cache_dir: Path,
}

impl Registry {
    pub fn new(cache_dir: &Path) -> Result<Self> {
        let index = SpdxIndex::load()
            .context("Failed to load SPDX license index")?;
        Ok(Self {
            index,
            cache_dir: cache_dir.to_path_buf(),
        })
    }

    pub fn get_template(&self, license_id: &str) -> Result<String> {
        let detail = SpdxIndex::fetch_detail(license_id, &self.cache_dir)
            .context(format!("Failed to fetch license template for {license_id}"))?;
        Ok(detail.license_text)
    }

    pub fn validate(&self, license_id: &str) -> Result<()> {
        if !self.index.is_valid(license_id) {
            anyhow::bail!("Unknown SPDX license ID: {license_id}");
        }
        if self.index.is_deprecated(license_id) {
            eprintln!("Warning: {license_id} is deprecated by SPDX");
        }
        Ok(())
    }

    pub fn search(&self, query: &str) -> Vec<&crate::spdx::SpdxLicense> {
        self.index.search(query)
    }
}
```

**Step 3:** Commit

```bash
git add src/registry.rs src/spdx.rs
git commit -m "feat: SPDX-based license fetching (replaces GitHub API)"
```

---

### Task 4: Add `licencify search` subcommand

**Objective:** Expose the 727-license search as a CLI command — free value from SPDX data.

**Files:**

- Modify: `src/cli.rs` (add `Search` subcommand)
- Modify: `src/main.rs` (handle search)

**Step 1:** Add to `src/cli.rs`

```rust
/// Search available licenses by name or ID
#[derive(Parser)]
pub struct Search {
    /// Search query (matches name and license ID)
    pub query: String,

    /// Show only OSI-approved licenses
    #[arg(long)]
    pub osi_only: bool,

    /// Show only FSF Libre licenses
    #[arg(long)]
    pub fsf_only: bool,
}
```

**Step 2:** Handle in `src/main.rs`

```rust
Commands::Search(args) => {
    let index = SpdxIndex::load()?;
    let results: Vec<_> = index.search(&args.query)
        .into_iter()
        .filter(|l| {
            if args.osi_only && !l.is_osi_approved { return false; }
            if args.fsf_only && !l.is_fsf_libre { return false; }
            true
        })
        .collect();

    if results.is_empty() {
        eprintln!("No licenses found matching '{}'", args.query);
        std::process::exit(1);
    }

    for l in &results {
        let mut flags = Vec::new();
        if l.is_osi_approved { flags.push("OSI"); }
        if l.is_fsf_libre { flags.push("FSF"); }
        if l.is_deprecated_license_id { flags.push("DEPRECATED"); }
        let flag_str = if flags.is_empty() { String::new() } else { format!(" [{}]", flags.join(", ")) };
        println!("  {:40s} {}{}", l.license_id, l.name, flag_str);
    }
    println!("\n{} licenses found", results.len());
}
```

**Step 3:** Commit

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: add 'licencify search' with OSI/FSF filters"
```

---

### Task 5: Update existing plan for SPDX-first architecture

**Objective:** Update the existing implementation plan to reflect SPDX as primary source.

**Files:**

- Modify: `.hermes/plans/2026-06-18_180000-licencify-cli-design.md`

**Step 1:** Update the architecture section and tier chain in the existing plan to reference SPDX instead of GitHub as Tier 2.

**Step 2:** Commit

```bash
git add .hermes/plans/
git commit -m "docs: update plan for SPDX-first architecture"
```

---

## Risks & Tradeoffs

| Risk                                                       | Mitigation                                                                                                         |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| SPDX endpoint goes down                                    | Built-in templates (Tier 3) always work; index embedded in binary                                                  |
| Per-license detail fetch is slow                           | Cache aggressively; popular licenses embedded as builtins                                                          |
| `licenseText` placeholder syntax varies                    | Use `licenseText` (not `standardLicenseTemplate`); simple `<year>`/`<copyright holders>` replace covers most cases |
| Binary size grows ~332KB from embedded index               | Acceptable for a CLI tool; gzip compresses well                                                                    |
| SPDX detail files may have inconsistent placeholder syntax | Handle gracefully: replace known placeholders, leave unknown ones as-is                                            |

## Open Questions

1. **Embed full index or lazy-load?** Embedding means zero-network validation but adds 332KB. Given this is a CLI tool, embedding is fine.
2. **Should we cache the index too?** Yes — add `licencify update-index` that re-fetches `licenses.json` to XDG cache, keeping the embedded version as a snapshot fallback.
3. **GitHub API as opt-in?** Could keep as a feature flag `--registry github` for users who prefer GitHub's formatting. Low priority.
4. **License text normalization?** Some SPDX texts have trailing whitespace or inconsistent line endings. Consider a `normalize_template()` step.
