# Licencify Product Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the approved product spec without changing its open decisions into implicit requirements.

**Architecture:** Deliver independently reviewable slices: shared configuration; SPDX sources and formats; safe file generation and manifest handling; read-only scan and README alignment. Preserve the existing Rust CLI/provider/resolution/project-handler boundaries; migrate callers together rather
than adding compatibility shims. Each slice ends with runnable CLI behavior and focused tests.

**Tech Stack:** Rust 2024, clap, serde/schemars, toml_edit, tera, ureq, existing Fs/MemFs, mise/hk.

**Spec:** `docs/superpowers/specs/2026-09-30-licencify-product-spec.md`

## Global Constraints

- Resolve project configuration from `${PRJ_CONFIG_HOME:-$PRJ_ROOT/.config}/licencify/config.toml` and optional `config.local.toml`; user config is `$XDG_CONFIG_HOME/licencify/config.toml` or the platform equivalent.
- Resolve SPDX cache from `${XDG_CACHE_HOME:-$HOME/.cache}/licencify/SPDX-Cache/` or the platform user cache directory; never migrate the old `api/` directory.
- `proprietary` is a Licencify config value, not an SPDX expression; mixing it with open-source additional IDs is invalid.
- Additional IDs produce missing separate `<BASENAME>-<SPDX-ID>.<actual-format>` files; existing extras are never overwritten; multiple licences never imply `AND` or `OR` in manifests.
- `--no-file` suppresses all licence-file writes; `--permit-promotion` and `--yes` have separate meanings.
- Use existing dependencies before introducing new ones. Tests using `global_fs` must hold `FsGuard`; isolate process CWD and XDG env in CLI smoke tests.
- Every AI-assisted commit includes `Co-authored-by: GPT-6 Sol via omp <noreply@openai.com>`; update the dated `.agents/logs/` entry as implementation proceeds.

---

### Task 1: Project Configuration and Provenance

**Files:** Modify `src/config.rs:8-100,110-389`, `src/commands/config_cmd.rs`, `src/cli.rs:162-169`, `src/lib.rs:20-71`; test in `src/config.rs` and an isolated CLI test in `tests/config_cli.rs` (create).

**Interfaces:** Produce `Config::load_effective(subdir: Option<&str>) -> Result<Config>`, `Config::project_path() -> Result<PathBuf>`, `Config::update_project_defaults(license: &str, author: &str, format: &str) -> Result<bool>` with spec semantics; preserve these existing signatures for callers.
Expose effective additional IDs through `Config.default.additional_licences: Option<Vec<String>>`. Carry source provenance alongside resolved values for `--verbose` rather than logging secret values.

- [ ] **Step 1: Write failing behavior tests.** Configure temporary `PRJ_ROOT`, `PRJ_CONFIG_HOME`, and `XDG_CONFIG_HOME`; assert that global-only config loads, shared `[[subdirs]]` at `docs` and `docs/api` compose shallow-to-deep, local defaults supersede shared subdirs, local matching subdir
      supersedes local defaults, and `[]` clears extras. Check component boundary (`docs/api-extra` must not match `docs/api`), invalid `../` and absolute subdir paths, ignored child config warning, invalid TOML error, unknown-key warning, and absence of a project config. Assert `config init` creates only
      the root shared config and does not replace it.
- [ ] **Step 2: Confirm failure.** Run `cargo test config -- --nocapture`; expect the new precedence/location tests to fail against the legacy `.licencify.toml` search.
- [ ] **Step 3: Implement the resolver.** In `config.rs`, select explicit absolute `PRJ_ROOT` (reject CWD outside), otherwise Git worktree root, otherwise highest ancestor with root shared config, otherwise CWD. Layer global/shared/shared matching subdirs/local/local matching subdirs by _field_ and
      preserve `None` versus `Some([])`. Derive the path relative to root from CWD by default. Accept `licence` as a TOML key via serde rename; reject invalid known values and proprietary/open-source mixtures after resolution. Warn on unknown fields without swallowing parse errors. Keep `schema_json()`
      derived from the updated schema; migrate `config show` and `init` to the new paths.
- [ ] **Step 4: Implement configuration writes.** Use `toml_edit::DocumentMut` to update only `licence`, `author`, and actual `format` in the selected winning root shared/local defaults or exact-path subdir table. In a child CWD prompt for shared-defaults versus exact subdir; with
      `--yes`/nonterminal choose subdir. Create shared config if absent; if local fields mask writes, update the winning local entry and report it. Reject child config writes. Record which file/rule supplied each field for verbose reporting without printing values.
- [ ] **Step 5: Verify real CLI behavior.** In a throwaway directory run `licencify config init`, `config show`, and `schema`; inspect root shared file and schema. Run `cargo test config` and `cargo test --test config_cli`; update CLI help and README config examples, then commit this slice.

### Task 2: SPDX Sources, Cache, and Output Formats

**Files:** Modify `src/provider.rs:15-89`, `src/commands/cache_cmd.rs:4-85`, `src/resolution.rs:6-115`, `src/cli.rs:15-29,35-128`, `src/template.rs`, `src/licences.rs`, `src/commands/add.rs`, `src/commands/update.rs`; test in the adjacent Rust test modules and `tests/template_cli.rs` (create). Modify `mise.toml:14-25` for the audit task.

**Interfaces:** Produce one shared cache-path function `provider::spdx_cache_dir() -> anyhow::Result<PathBuf>` consumed by `LicenseProvider::load()` and cache commands; keep `with_api_cache(&Path)` as the existing injectable test constructor but rename it and migrate its callers as part of this
slice. Extend `ResolvedTemplate` with actual output format and source; all filename consumers use that format rather than the requested format.

- [ ] **Step 1: Write failing tests.** With temporary XDG cache, assert provider and `cache info` resolve the same `licencify/SPDX-Cache` directory; `cache clear` removes only that subtree. Cache a detail with text but no HTML, add distinct project/global templates, and assert project then global
      beats cached content, cached detail suppresses network, HTML and Markdown use HTML only when available, and missing HTML warns and creates `.txt` rather than mislabeled `.html`/`.md`. Assert malformed chosen custom templates fail instead of falling through, and a fetched response remains usable if
      cache writing fails.
- [ ] **Step 2: Confirm failure.** Run `cargo test provider`, `cargo test resolution`, and `cargo test --test template_cli`; expect the new path/precedence/format cases to fail.
- [ ] **Step 3: Implement one cache path.** Use `dirs::cache_dir()?.join("licencify").join("SPDX-Cache")`; make provider load, `cache info`, `clear`, and `fetch-all` use it. Propagate clear failures, report directory size, and parse a successful fetched response before optional cache write so a write error cannot discard it. Leave old `api/` untouched.
- [ ] **Step 4: Implement source and format resolution.** Validate SPDX ID before inserting it in a path or URL. For text/HTML/Markdown, try project custom `<id>.tera`/`<id>.html.tera`, global equivalent, cached detail, fetched detail, then bundled template. Proprietary goes project/global/bundled
      only; never touch the SPDX network/cache. Preserve `licenseTextHtml` versus `licenseText` and convert rendered HTML to Markdown without losing visible words; if no HTML source, warn and choose text plus actual `txt` extension. Render Tera/SPDX placeholders exactly once.
- [ ] **Step 5: Wire formats and audit.** Add `md` to clap `LicenseFormat`, propagate resolved actual extension through `add`/`update`, and add `[tasks.audit]` running `cargo audit` to `mise.toml` (and the managed tool only if absent). Exercise `licencify cache info`, offline `add MIT --format
html`, `add MIT --format md` in a throwaway project; run focused tests and `mise run audit`, then commit with docs/help changes.

### Task 3: Safe Add/Update, Extra Files, and Proprietary Manifests

**Files:** Modify `src/cli.rs:32-128`, `src/lib.rs:20-71`, `src/commands/add.rs`, `src/commands/update.rs`, `src/licence_name.rs`, `src/project/mod.rs`, `src/project/handler.rs`, `src/project/cargo.rs`, `src/project/npm.rs`, `src/project/python.rs`, `src/fs.rs` (only if safe rename/remove operations
are missing); create `src/commands/generate.rs` only if it replaces duplicate add/update logic; test in `tests/generation_cli.rs` (create) and existing module tests.

**Interfaces:** One shared generation path consumes effective `Config`, selected primary ID, rendered primary and additional outputs, and explicit CLI switches; it returns paths actually written and skipped operations. Manifest handlers receive the selected ID and optional _existing/generated
primary filename_ rather than conflating `proprietary` with `UNLICENSED`. Do not make `--no-file` fabricate a notice path.

- [ ] **Step 1: Write failing behavior tests.** Assert `add` without ID uses effective config and errors if absent; `update` requires explicit ID; both generate missing extra files with selected author/year and correct actual extension; an existing extra with another extension survives untouched;
      invalid extra or unresolvable template prevents primary mutation; a write error reports partial failure; `--no-file` writes no primary or extra files. Assert multiple primary candidates fail without changes, changing format removes the old primary only after replacement succeeds, and promotion
      requires `--permit-promotion`, preserves source format, removes exactly one matching extra and updates config after success; `--yes` affects prompts only.
- [ ] **Step 2: Confirm failure.** Run `cargo test --test generation_cli`; expect missing CLI flags, missing extras, and unsafe replacement failures.
- [ ] **Step 3: Implement preflight and safe writes.** Resolve effective config once without `.ok()`; validate all IDs/paths, locate all basename/extension variants, resolve and render all missing outputs before mutation, reject ambiguous primary/promotion cases. Stage replacement before removing
      old primary; on failed move/config update preserve original primary and promotion source. Use exclusive creation for additional files so an existing file cannot be overwritten by a race. Report every output and any partial write failure accurately. Maintain additional-list precedence and remove
      promoted ID only after safe transition.
- [ ] **Step 4: Implement manifest semantics.** Skip every manifest with a warning if extras exist. For Cargo proprietary set `publish = false`, remove `license`, set `license-file` to actual notice; npm set `private = true` and `SEE LICENSE IN <filename>`; Python remove `project.license`, include
      the notice using `license-files` where supported, and warn publication needs independent control. When returning to SPDX, set the valid SPDX representation and warn without clearing Cargo/npm private flags. With `--no-file`, require an existing proprietary primary notice before referencing it.
      Propagate malformed/failed manifest paths rather than treating them as success.
- [ ] **Step 5: Exercise the binary.** In an isolated directory set shared config with `licence = "MIT"` and `additional-licences = ["Apache-2.0"]`; run `add --yes`, check both files and skipped manifest update, rerun and check extra unchanged; exercise `update proprietary` only after removing
      extras and inspect each ecosystem manifest. Run focused tests plus `cargo test`, update README/CLI docs, and commit.

### Task 4: Read-Only Scan, Detection, and README Consistency

**Files:** Create `src/commands/scan.rs` and register it in `src/commands/mod.rs`; modify `src/cli.rs:96-100`, `src/lib.rs:45-69`, `src/detect.rs`, `src/commands/detect_cmd.rs`, `src/readme.rs`, `src/config.rs` (the `[scan]` exclusion schema); create `tests/scan_cli.rs`; update `README.md`.

**Interfaces:** `commands::cmd_scan(expected_primary: Option<&str>) -> anyhow::Result<()>` is read-only; consume effective configuration for each evidence directory, shared/local exclusions and explicit scan primary override. README updates receive actual generated primary filename, not a fixed `LICENCE.txt`.

- [ ] **Step 1: Write failing scan tests.** In isolated temporary Git projects create root and nested `LICENSE`, `LICENCE-MIT.txt`, `COPYING.md`, README declaration, ignored `vendor`, configured `[[subdirs]]`, a missing configured directory, a mismatched filename/content pair, and an unreadable
      file. Assert all findings have path and evidence level; unknown text is not assigned an ID; confirmed conflicts/unreadable files return nonzero; unknown/missing expected items warn; overlapping exclusion/subdir warns and still scans the configured subdir; scan changes no files. Assert Apache
      generic “version 2.0” alone does not identify Apache.
- [ ] **Step 2: Confirm failure.** Run `cargo test --test scan_cli`; expect missing command and failed classifications.
- [ ] **Step 3: Implement scan.** Traverse from selected project root, honoring Git ignore and normalized project-relative `[scan] exclude`; do not follow symlink escapes. Identify conventional primary/ID-suffixed extra filenames, call `detect::detect_license` for distinctive text, parse only
      explicit README declarations, badges, and licence links. Keep claims distinct from confidently identified text; compare with effective root/subdir expectations and optional primary override. Return nonzero on confirmed contradiction/read error, warnings otherwise; keep `detect` independent.
- [ ] **Step 4: Fix README updates.** Pass actual output filename to badge/section rendering. Recognize and replace only tool-managed badge/section, preserve handwritten content and warn about stale manual references; accept both `--update-readme` and `--no-update-readme` to override configuration. Keep absent/unchanged README a successful no-op and report write failure distinctly.
- [ ] **Step 5: Smoke and release checks.** Run real `licencify scan` on a temporary multi-licence project and check its exit code and output; run `cargo test --test scan_cli`, full `cargo test`, `cargo fmt --check`, scoped `hk run pre-commit` after inspecting planned effects, and `git diff --check`. Update CLI help/README, append the dated AI log, and commit.

## Open decisions (do not silently implement)

The spec leaves SPDX index refresh/versioning, a `config set` command, the manifest-update default, a proprietary `remove` command, and remote registries open. Keep the current manifest-update default until the maintainer chooses otherwise; do not add the other commands/protocols in these tasks. If
a task requires a choice (for example a new manifest-update flag), get that decision before implementing that step.
