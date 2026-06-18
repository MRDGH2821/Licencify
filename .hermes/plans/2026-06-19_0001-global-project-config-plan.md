# Global + Project-Level Config with Sub-Directory Licensing

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** Add two-tier config (global + project) with project-level sub-directory licensing support, so a monorepo can assign different licenses to different subdirectories.

**Architecture:**

- **Global config** (`~/.config/licencify/config.toml`) — user-wide defaults
- **Project config** (`./licencify.toml`) — project-level overrides + `[subdirs]` section
- Merge: project overrides global; `[subdirs]` matched by longest-prefix path against CWD
- Config set always writes to global; project config is hand-edited

**Tech Stack:** Rust, serde, toml, toml_edit, schemars (existing deps)

---

## Current State

| File                         | Status                              |
| ---------------------------- | ----------------------------------- |
| `src/config.rs`              | Single global config only, no merge |
| `src/commands/config_cmd.rs` | Works with single config            |
| `src/resolution.rs`          | Loads single config                 |
| `src/author.rs`              | Loads single config                 |
| `src/commands/add.rs`        | Uses `Config::load()`               |
| `src/commands/update.rs`     | Uses `Config::load()`               |

---

## Config Format Design

### Global config (`~/.config/licencify/config.toml`)

```toml
[default]
author = "Mihir Rabade"
license = "MIT"
format = "html"
year = "2026"
licence_name = "LICENCE"

[template]
paths = ["./global-templates"]
```

### Project config (`./licencify.toml`)

```toml
[default]
author = "Mihir Rabade"
license = "Apache-2.0"
format = "txt"

[template]
paths = ["./project-templates"]

[subdirs]
# Longest-prefix match against CWD determines which subdir config applies
# Each subdir entry has its own default overrides
[subdirs."packages/backend"]
license = "MIT"
licence_name = "LICENSE"

[subdirs."packages/frontend"]
license = "MIT"

[subdirs."packages/docs"]
license = "CC-BY-4.0"
licence_name = "LICENCE"
```

### Resolution logic

```
merged_config = merge(global, project)
effective_config = merge(merged_config, matched_subdir_config)

Match algorithm:
  1. Get CWD relative to project root (dir containing licencify.toml)
  2. Find longest prefix match in [subdirs]
  3. Merge matched subdir config on top of merged global+project
```

---

## Tasks

### Task 1: Add `SubdirConfig` struct and `[subdirs]` to Config

**Objective:** Extend the config model to support sub-directory license overrides.

**Files:**

- Modify: `src/config.rs`

**Step 1: Add SubdirConfig struct**

```rust
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
pub struct SubdirConfig {
    pub default: Option<DefaultConfig>,
}
```

**Step 2: Add `subdirs` field to Config**

```rust
pub struct Config {
    #[schemars(description = "Default values for licence creation")]
    pub default: DefaultConfig,

    #[schemars(description = "Template configuration")]
    pub template: Option<TemplateConfig>,

    #[schemars(description = "Sub-directory license overrides (key = relative path)")]
    pub subdirs: Option<std::collections::HashMap<String, SubdirConfig>>,
}
```

**Step 3: Add `Default for Config`**

```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            default: DefaultConfig::default(),
            template: None,
            subdirs: None,
        }
    }
}
```

**Step 4: Build and verify**

Run: `cargo build 2>&1`
Expected: builds clean

**Step 5: Commit**

```bash
git add src/config.rs
git commit -m "feat: add SubdirConfig struct and [subdirs] config section"
```

---

### Task 2: Add config merge function

**Objective:** Implement deep merge where `overriding` values take priority over `base`.

**Files:**

- Modify: `src/config.rs`

**Step 1: Implement `merge` function**

```rust
/// Merge two configs: `overriding` values take priority over `base`.
/// Only `Some` values in `overriding` replace `base`.
fn merge(base: Config, overriding: Config) -> Config {
    Config {
        // DefaultConfig: field-by-field merge
        default: DefaultConfig {
            author: overriding.default.author.or(base.default.license),
            license: overriding.default.license.or(base.default.license),
            format: overriding.default.format.or(base.default.format),
            year: overriding.default.year.or(base.default.year),
            licence_name: overriding.default.licence_name.or(base.default.licence_name),
        },
        // Template: overriding paths replace entirely if present
        template: match (base.template, overriding.template) {
            (Some(base_t), Some(over_t)) => {
                let paths = over_t.paths.or(base_t.paths);
                Some(TemplateConfig { paths })
            }
            (None, Some(t)) => Some(t),
            (Some(t), None) => Some(t),
            (None, None) => None,
        },
        // Subdirs: overriding replaces entirely if present
        subdirs: overriding.subdirs.or(base.subdirs),
    }
}
```

**Step 2: Build and verify**

Run: `cargo build 2>&1`
Expected: builds clean

**Step 3: Commit**

```bash
git add src/config.rs
git commit -m "feat: add config merge function for global + project overlay"
```

---

### Task 3: Split `Config::load()` into `load_global()` + `load_project()`

**Objective:** Load global and project configs independently, then merge.

**Files:**

- Modify: `src/config.rs`

**Step 1: Add path helpers**

```rust
impl Config {
    /// Return the global config file path.
    pub fn global_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("Could not determine config directory")?;
        Ok(config_dir.join("licencify").join("config.toml"))
    }

    /// Return the project-level config file path (current directory).
    pub fn project_path() -> Result<PathBuf> {
        Ok(std::env::current_dir()
            .context("Could not determine current directory")?
            .join("licencify.toml"))
    }

    /// Legacy path() — returns global path for backwards compat.
    pub fn path() -> Result<PathBuf> {
        Self::global_path()
    }

    fn load_global() -> Option<Self> {
        let path = Self::global_path().ok()?;
        if !path.exists() { return None; }
        let contents = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&contents).ok()
    }

    fn load_project() -> Option<Self> {
        let path = Self::project_path().ok()?;
        if !path.exists() { return None; }
        let contents = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&contents).ok()
    }

    /// Load merged config: global + project-level (project overrides global).
    pub fn load() -> Result<Self> {
        let global = Self::load_global().unwrap_or_default();
        let project = Self::load_project().unwrap_or_default();
        Ok(merge(global, project))
    }

    /// Save always writes to global config.
    pub fn save(&self) -> Result<()> {
        let path = Self::global_path()?;
        // ... existing save logic, write to global_path ...
    }
}
```

**Step 2: Build and verify**

Run: `cargo build 2>&1 && cargo test 2>&1`
Expected: builds clean, tests pass

**Step 3: Commit**

```bash
git add src/config.rs
git commit -m "feat: load and merge global + project config files"
```

---

### Task 4: Add subdir resolution logic

**Objective:** Match CWD against `[subdirs]` longest-prefix and produce effective config.

**Files:**

- Modify: `src/config.rs`

**Step 1: Add `load_effective()` and subdir matching**

```rust
impl Config {
    /// Load the effective config, including subdir resolution.
    /// 1. Merge global + project
    /// 2. Find project root (dir containing licencify.toml)
    /// 3. Compute relative CWD from project root
    /// 4. Find longest-prefix match in [subdirs]
    /// 5. Merge matched subdir config on top
    pub fn load_effective() -> Result<Self> {
        let merged = Self::load()?;

        let project_path = Self::project_path()?;
        let project_root = project_path
            .parent()
            .context("Could not determine project root")?;

        let cwd = std::env::current_dir()
            .context("Could not determine current directory")?;

        let rel = cwd.strip_prefix(project_root).unwrap_or(&cwd);
        let rel_str = rel.to_string_lossy();

        let subdirs = match &merged.subdirs {
            Some(s) => s,
            None => return Ok(merged),
        };

        // Find longest prefix match
        let best = subdirs
            .iter()
            .filter(|(key, _)| rel_str.starts_with(key.as_str()))
            .max_by_key(|(key, _)| key.len());

        if let Some((_, subdir_cfg)) = best {
            let default_override = subdir_cfg.default.clone().unwrap_or_default();
            // Build an overriding Config with just the subdir's default
            let override_config = Config {
                default: default_override,
                template: None,
                subdirs: None,
            };
            Ok(merge(merged, override_config))
        } else {
            Ok(merged)
        }
    }
}
```

**Step 2: Update `save()` to target global path** (if not already done in Task 3)

**Step 3: Build and verify**

Run: `cargo build 2>&1`
Expected: builds clean

**Step 4: Commit**

```bash
git add src/config.rs
git commit -m "feat: add subdir resolution with longest-prefix matching"
```

---

### Task 5: Update all consumers to use `load_effective()`

**Objective:** All config reads should go through `load_effective()` so subdir overrides apply.

**Files:**

- Modify: `src/resolution.rs` — `resolve_year()`, `resolve_author()`, `resolve_template()`
- Modify: `src/author.rs` — `ConfigAuthorResolver`
- Modify: `src/commands/add.rs` — `Config::load()` → `Config::load_effective()`
- Modify: `src/commands/update.rs` — same

**Step 1: In `src/resolution.rs`**

Change all `Config::load()` calls to `Config::load_effective()`:

```rust
// resolve_year
if let Ok(cfg) = Config::load_effective() {

// resolve_template
let config = Config::load_effective().ok();
```

**Step 2: In `src/author.rs`**

```rust
let cfg = crate::config::Config::load_effective().ok()?;
```

**Step 3: In `src/commands/add.rs`**

```rust
let config = Config::load_effective().ok();
```

**Step 4: In `src/commands/update.rs`**

```rust
let config = Config::load_effective().ok();
```

**Step 5: Build and verify**

Run: `cargo build 2>&1 && cargo test 2>&1`
Expected: builds clean, tests pass

**Step 6: Commit**

```bash
git add src/resolution.rs src/author.rs src/commands/add.rs src/commands/update.rs
git commit -m "refactor: use load_effective() everywhere for subdir-aware config"
```

---

### Task 6: Update config_cmd.rs for global + project awareness

**Objective:** `config show` displays both levels; `config init` creates project config; `config set` writes global.

**Files:**

- Modify: `src/commands/config_cmd.rs`

**Step 1: Update `cmd_config_init()` to create project config**

```rust
fn cmd_config_init() -> Result<()> {
    let project_path = Config::project_path()?;
    if project_path.exists() {
        println!("Project config already exists: {}", project_path.display());
        return Ok(());
    }

    let cfg = Config::default();
    let doc: toml_edit::DocumentMut = toml::to_string_pretty(&cfg)
        .expect("serialize")
        .parse()
        .expect("parse");

    std::fs::write(&project_path, doc.to_string())?;
    println!("✅ Created project config: {}", project_path.display());

    // Ensure global exists too
    let global_path = Config::global_path()?;
    if !global_path.exists() {
        let global_cfg = Config::default();
        global_cfg.save()?;
        write_schema_file()?;
    }

    println!();
    println!("Available settings:");
    println!("  [default]");
    println!("    author, license, format, year, licence_name");
    println!("  [template]  (optional)");
    println!("    paths = []");
    println!("  [subdirs]   (optional, project config only)");
    println!("    [subdirs.\"path/to/subdir\"]");
    println!("      license = \"MIT\"");
    println!();
    println!("Config precedence:");
    println!("  project (./licencify.toml) > global (~/.config/licencify/)");
    println!("  subdir overrides apply on top within the project.");
    Ok(())
}
```

**Step 2: Update `cmd_config_show()` to show both levels**

```rust
fn cmd_config_show() -> Result<()> {
    let global_path = Config::global_path()?;
    let project_path = Config::project_path()?;
    let effective = Config::load_effective()?;

    println!("Config resolution:");
    println!("  Global:   {} {}", global_path.display(),
        if global_path.exists() { "✓" } else { "✗" });
    println!("  Project:  {} {}", project_path.display(),
        if project_path.exists() { "✓" } else { "✗" });
    println!();

    // Show effective values
    let detected = config::detect_licence_name();
    println!("[default]  (effective values)");
    println!("  author        = {}", effective.default.author.as_deref().unwrap_or("(not set)"));
    println!("  license       = {}", effective.default.license.as_deref().unwrap_or("(not set)"));
    println!("  format        = {}", effective.default.format.as_deref().unwrap_or("(not set)"));
    println!("  year          = {}", effective.default.year.as_deref().unwrap_or("(not set)"));
    println!("  licence_name  = {} (detected: {})",
        effective.default.licence_name.as_deref().unwrap_or("(not set)"), detected);

    // Show project-only sections
    if project_path.exists() {
        if let Some(subdirs) = &effective.subdirs {
            println!();
            println!("[subdirs]  (project config only)");
            for (path, cfg) in subdirs {
                println!("  [\"{}\"]", path);
                if let Some(default) = &cfg.default {
                    if let Some(lic) = &default.license {
                        println!("    license = {}", lic);
                    }
                }
            }
        }
    }
    Ok(())
}
```

**Step 3: Update `cmd_config_set()` to write to global**

Already writes to `Config::save()` which targets global. Update the output message:

```rust
fn cmd_config_set(key: &str, value: &str) -> Result<()> {
    let mut config = Config::load()?;  // load merged
    // ... existing match logic ...
    config.save()?;  // saves to global
    let path = Config::global_path()?;
    println!("✅ Set {} = {}", key, value);
    println!("   Saved to {}", path.display());
    Ok(())
}
```

**Step 4: Build and verify**

Run: `cargo build 2>&1 && cargo test 2>&1`
Expected: builds clean, tests pass

**Step 5: Commit**

```bash
git add src/commands/config_cmd.rs
git commit -m "feat: config cmd shows both global/project levels and subdir overrides"
```

---

### Task 7: Add schema support for `[subdirs]`

**Objective:** The `schema` command and `#:schema` comment should reflect the subdirs section.

**Files:**

- Modify: `src/config.rs` (already has `SubdirConfig` with `JsonSchema` derive)
- Modify: `src/commands/config_cmd.rs` — `write_schema_file()` now also writes to project dir

**Step 1: Update `write_schema_file()` to write schema alongside project config too**

```rust
fn write_schema_files() -> Result<()> {
    // Global schema
    let global_schema_path = Config::schema_path()?;
    let json = Config::schema_json()?;
    if let Some(parent) = global_schema_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&global_schema_path, &json)?;
    println!("✅ Schema written: {}", global_schema_path.display());

    // Project schema (if project config exists)
    let project_path = Config::project_path()?;
    if project_path.exists() {
        let project_schema = project_path.with_file_name("licencify-schema.json");
        std::fs::write(&project_schema, &json)?;
        println!("✅ Schema written: {}", project_schema.display());
    }
    Ok(())
}
```

**Step 2: Build and verify**

Run: `cargo build 2>&1`
Expected: builds clean

**Step 3: Commit**

```bash
git add src/commands/config_cmd.rs
git commit -m "feat: generate schema files for both global and project config"
```

---

### Task 8: End-to-end verification

**Objective:** Verify the full flow works correctly.

**Step 1: Test global config**

```bash
rm -rf ~/.config/licencify
cargo run -- config init
cat ~/.config/licencify/config.toml
cargo run -- config set author "Mihir Rabade"
cargo run -- config set licence_name LICENCE
cargo run -- config show
```

Expected: global config created, set/show works

**Step 2: Test project config with subdirs**

```bash
# Create a project config with subdir overrides
cat > licencify.toml << 'EOF'
[default]
author = "Mihir Rabade"
license = "Apache-2.0"

[subdirs."packages/backend"]
license = "MIT"

[subdirs."packages/docs"]
license = "CC-BY-4.0"
EOF

cargo run -- config show
# Should show effective values, with subdir info
```

**Step 3: Test subdir resolution**

```bash
mkdir -p packages/backend
cd packages/backend
# Verify that the subdir override applies
cargo run -- config show
# Should show license = "MIT" (from subdir override)
cd ../..
```

**Step 4: Run existing tests**

```bash
cargo test 2>&1
```

**Step 5: Commit**

```bash
git add -A
git commit -m "chore: end-to-end verification of global + project config"
```

---

## Risks and Trade-offs

1. **`config set` always writes to global** — project config is manually edited. This is intentional: project configs are version-controlled and should be reviewed.

2. **Longest-prefix matching for subdirs** — simpler than glob/regex, sufficient for monorepo patterns like `packages/backend`.

3. **Schema file duplicated** — generated alongside both global and project configs. Could be a symlink, but duplication is simpler.

4. **Breaking change** — `Config::load()` now reads from two locations. If a project doesn't have `licencify.toml`, behavior is unchanged (only global is used).

---

## Open Questions

- Should `config init` accept a `--global` flag to init global config explicitly, and `--project` (or no flag) for project config? Currently `config init` creates project, `config set` writes global.
