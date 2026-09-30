# Licencify Product Specification (Draft)

**Status:** Draft for maintainer review. This consolidates requirements and proposals from the plans in this directory; it is not an implementation plan.

## Purpose

Licencify is a command-line tool for selecting an open-source licence, rendering its text for a project, and optionally recording the licence in project metadata. It uses the SPDX licence list as its discovery and metadata source, with local templates and cached/fetched SPDX detail text for generation.

## Goals

- Make licence discovery and metadata lookup available offline.
- Generate plain-text, HTML, or Markdown licence files with user/project-specific copyright details.
- Support common licences offline and broader SPDX coverage online.
- Apply project defaults consistently across global, project, and subdirectory configuration.
- Update project manifests safely when requested.
- Make repeated and scripted use predictable, and report failures without silently losing user data.

## User-facing requirements

### Licence discovery

1. The embedded SPDX index is the source of valid licence identifiers and metadata. It includes each entry's name, deprecation state, OSI/FSF status, and references where supplied by SPDX.
2. `licencify list` lists available identifiers and supports OSI/FSF filters and a result limit.
3. `licencify search <query>` searches identifiers and names and supports the same filters. Discovery works without network access.
4. Deprecated identifiers are identified to the user rather than treated as ordinary current identifiers.

### Add and update

1. `licencify add [ID]` generates the primary licence in the current directory; an omitted ID uses the effective `licence` setting, and absence of both is an error. `licencify update <ID>` requires an explicit replacement ID. Both commands manage only the primary licence, not files listed in `additional-licences`.
2. Both commands accept explicit author, company, email, year, and `txt`/`html`/`md` format options. An explicitly supplied CLI value overrides every configuration layer.
   Unset format falls back to `txt`; unset year uses the current year, company defaults to author, email is empty, README updates default to false, and the filename convention defaults to the detected locale. An unresolved author is an actionable error.
3. Supported template variables are `year`, `author`, `company`, `email`, and `date`. SPDX placeholders used in source text are rendered when recognized; unrecognized placeholders are not silently erased.
4. Plain-text output uses the template source order specified below. Custom templates use `<spdx-id>.tera` for text and `<spdx-id>.html.tera` for HTML. Markdown output converts rendered HTML to Markdown while preserving visible licence wording. If no HTML source exists for requested `md` or `html`, warn and write plain text with a `.txt` extension instead; fail if no plain-text source exists either.
5. `update` locates the single existing primary licence file across the supported basename and format variants, confirms replacement, and removes or renames the old path only after the replacement is ready. It fails without changing files if multiple primary candidates make the target ambiguous. `--yes` skips confirmation prompts; scripted invocations must not wait for input.
6. Output naming respects `licence_file_name = "licence"` or `"license"` (case-insensitive), rendered as uppercase `LICENCE` or `LICENSE`, and the actual output format's extension. Additional licences are separate, manually maintained files named `<BASENAME>-<SPDX-ID>.<format>`, such as `LICENCE-Apache-2.0.txt`.
7. If the requested primary ID is in `additional-licences`, reject the command unless `--permit-promotion` is supplied. Promotion moves exactly one existing matching additional file into the primary position, removes the old primary, removes the promoted ID from the effective additional list, and records it as primary.
   Do not silently relabel a source format, guess among multiple source files, or delete the old primary before the move and config change can succeed. `--permit-promotion` authorizes this transition; `--yes` separately controls prompts.
8. The commands report which file was written, the selected licence, and any manifest/readme updates or skipped operations. With `--verbose`, report each resolved setting's source (CLI, config file and matching rule, or fallback) without printing its value.

### Detection

1. `licencify detect` checks recognized licence filenames in the current project and attempts to identify supported licence text using distinctive signatures.
2. A single generic phrase such as “version 2.0” is insufficient to identify a licence. Apache-2.0 detection requires both an Apache-specific and version-specific signature.
3. The detector recognizes the built-in licence families documented by the CLI, including proprietary notices as `UNLICENSED`.
4. Missing files, unreadable files, and unrecognized contents are reported as errors or no-match outcomes; they do not terminate the process through an internal process exit.

### Project scan

1. `licencify scan` recursively discovers conventional `LICENCE`, `LICENSE`, and `COPYING` files (including `.txt`, `.md`, and `.html` variants and ID-suffixed additional files) beneath the selected project root. Unlike `detect`, it reports every result rather than stopping at the first.
   It also examines READMEs for explicit licence declarations, badges, and licence-file links, not incidental mentions in examples.
2. Skip Git-ignored directories and project-relative directories listed in project configuration under `[scan] exclude = ["vendor"]`. Exclusion paths cannot escape the project root; do not follow directory symlinks outside it. If an exclusion overlaps a configured `[[subdirs]]` path, warn that the configuration is contradictory and scan that subdir anyway.
3. For each result, show its path and evidence level: identified from distinctive text, claimed by a filename or README declaration, or unidentified. An unrecognized file is reported as unknown, not assigned a guessed SPDX ID. Report every file in a multi-licence directory and conflicts between confidently identified text and explicit file/README claims.
4. Discovery works without any configuration. If configuration exists, compare findings against effective `licence` and `additional-licences` at the root, configured subdir paths, and directories with discovered evidence. An optional explicit scan ID overrides the expected primary ID, not the additional list. Traversal does not require every visited directory to have a licence file.
5. A confirmed conflicting ID or unreadable file causes a nonzero result; unknown identity, missing expected licence file or directory, and contradictory exclusions cause warnings. A missing configured path is not silently reported as matching. The scan is read-only and does not run automatically after `add` or `update`.

### Configuration

1. Select the project root from an absolute `PRJ_ROOT` when set, otherwise the Git worktree root, otherwise the highest ancestor containing a project config, otherwise the current directory. Reject an invocation whose current directory lies outside an explicit `PRJ_ROOT`. `PRJ_CONFIG_HOME`, when set, replaces `$PRJ_ROOT/.config` as the project config directory.
2. Load global defaults from `$XDG_CONFIG_HOME/licencify/config.toml` (or the platform equivalent), then the root shared config at `${PRJ_CONFIG_HOME:-$PRJ_ROOT/.config}/licencify/config.toml`, then its optional `config.local.toml`. The local file is intended to remain uncommitted. Legacy root-level config filenames are not loaded.
3. Only root project config files are loaded. When invoked from a child directory containing a Licencify config file on the path from the root to the current directory, warn that it was ignored and recommend merging it into the root shared config. Do not search the whole project tree for child configs.
4. `[default]` supports optional `author`, `company`, `email`, `year`, `licence` (one SPDX ID), `additional-licences` (an array of other SPDX IDs), `format`, `update_readme`, and `licence_file_name`. Project files may additionally contain `[[subdirs]]` entries with a required `path` and optional overrides for any of these fields; global `[[subdirs]]` entries are invalid.
   Subdir paths are normalized relative to the project root, cannot escape it, and match only complete path components. A later `additional-licences` array replaces an earlier one; `[]` clears inherited additional IDs.
5. Resolve fields in this order, lowest to highest: global defaults; shared project defaults; matching shared subdirs from shallowest to deepest; local project defaults; matching local subdirs from shallowest to deepest; explicit CLI options.
   Later sources override only fields they specify. Entries for the same normalized path in shared and local files contribute only their explicit fields at their respective precedence positions. A local default therefore overrides a shared subdir value; a local subdir can override it again.
6. If a required licence or author remains unresolved after CLI and configuration lookup, report an actionable error. Missing optional fields use the fallbacks specified under Add and update; neither a global config file nor every optional field is mandatory. Invalid TOML and invalid values of known fields are errors. Warn with file and key for unknown config options and ignore those options.
7. `config init` creates only the root shared config at `${PRJ_CONFIG_HOME:-$PRJ_ROOT/.config}/licencify/config.toml` without overwriting it; it does not create the global or local file. `config show` identifies loaded files and effective values. `schema` writes a JSON schema for the supported keys to the requested path.
8. `add` and `update` record the chosen primary ID, author, and actual format in configuration, preserving unrelated values and comments. From the root, write the shared defaults. From a subdirectory, prompt interactively for shared defaults or an exact-path `[[subdirs]]` entry; an explicit config-target CLI option decides without prompting. With `--yes` or no terminal, default to the subdir entry.
   Create the root shared config if the chosen subdir target needs one and none exists. If a local override masks the fields being saved, alert the user and update the winning local entry so future resolutions see the new values. Do not write child config files.

Config and template locations:

```text
Global:  $XDG_CONFIG_HOME/licencify/config.toml
         $XDG_CONFIG_HOME/licencify/templates/*.tera
Project: ${PRJ_CONFIG_HOME:-$PRJ_ROOT/.config}/licencify/config.toml
         ${PRJ_CONFIG_HOME:-$PRJ_ROOT/.config}/licencify/config.local.toml
         ${PRJ_CONFIG_HOME:-$PRJ_ROOT/.config}/licencify/templates/*.tera
```

For example, `[[subdirs]]` entries use TOML string paths such as `path = "docs/api"`; a matching `path = "docs"` provides inherited fields unless a more-specific entry overrides them.

The `[scan]` exclusion list belongs in project config; the local list adds to the shared list. Configured `[[subdirs]]` paths still take priority for scan coverage when they overlap an exclusion.

### Templates, cache, and network

1. The embedded SPDX index supports validation, listing, searching, and metadata without network access; it is not part of the runtime template cache.
2. Cache location follows PRJ Base Directory Specification v0.1: an absolute `PRJ_CACHE_HOME` environment value takes precedence. Otherwise, use `${XDG_CACHE_HOME:-$HOME/.cache}/prj/$PRJ_ID` when `PRJ_ID` is available; otherwise use `$PRJ_ROOT/.cache`.
3. `PRJ_ID` is taken from its environment variable when set; otherwise Licencify reads `prj_id` in the effective project config home, if present, and strips trailing whitespace. It must match `^[A-Za-z0-9_-]{0,32}$`; an invalid value is an error. `XDG_CACHE_HOME` defaults to `$HOME/.cache` when unset.
4. Licencify stores its cache data under `$PRJ_CACHE_HOME/licencify/`, keeping it separate from other tools' project cache data. When no project identifier exists, that path is project-local under `$PRJ_ROOT/.cache/licencify/`.
5. On an SPDX detail cache hit, use that detail without a network request; if its HTML field is absent, continue to bundled HTML or the plain-text fallback rather than refetching the same detail. On a cache miss, fetch the detail and cache the successful response. Cached detail remains available until `cache clear`; this specification defines no time-based expiry.
6. A cache write failure does not discard a successfully fetched response. A network failure may fall back to a built-in template where available; otherwise report an actionable error.
7. `cache info` reports the resolved Licencify cache directory and size. `cache clear` removes only `$PRJ_CACHE_HOME/licencify/`; clearing the cache must not delete configuration or other project data.
8. Built-in templates provide an offline generation path for the supported common licences.
9. For each requested format, search project templates, global templates, cached SPDX detail, fetched SPDX detail on a cache miss, then bundled templates, in that order. HTML and Markdown use the SPDX detail's `licenseTextHtml` only when present; plain text uses `licenseText`.
   A custom template wins even when SPDX detail is already cached. A failed fetch may still use a bundled template. Missing HTML follows the warned `.txt` fallback above. A template that exists but cannot be read or rendered fails visibly rather than silently selecting another source.

### Manifest integration

1. When manifest updates are enabled, Licencify detects supported manifests and writes the selected SPDX identifier using the ecosystem's supported representation. If the effective `additional-licences` array is nonempty, skip writing a single-ID manifest declaration and warn rather than misrepresent a multi-licence project.
2. Initial target manifests are `Cargo.toml`, `package.json`, and `pyproject.toml`. Existing formatting/comments should be preserved where the file format permits.
3. An unsupported or malformed manifest is reported with its path. Failure to update one manifest must not be reported as success for that manifest.
4. Proprietary declarations map to `UNLICENSED` in manifests. For npm, whether the tool also sets `private = true` requires maintainer confirmation.
5. `--no-file` skips writing a licence text file while retaining explicitly enabled manifest updates. The plans recommend that it work with any valid supported licence ID.

### Proprietary declaration

1. `proprietary` is a Licencify special value, not an SPDX licence-list entry; it maps to `UNLICENSED` for manifest representation and detection.
2. The proposed generated notice is a short copyright / “All rights reserved” notice, not a full EULA or legal document.
3. The current proposal defaults `licencify add proprietary` to writing that notice; `--no-file` permits manifest-only operation.
4. A `remove` command that deletes a licence file and sets manifests to `UNLICENSED` is an optional proposal, not an accepted requirement in the source plans.

### README update

1. README modification is opt-in by CLI or configuration; the default is no change.
2. When enabled, Licencify locates a supported Markdown README, adds a licence badge and licence section only when an equivalent one is not already present, and leaves non-Markdown README formats unchanged. On a licence change, replace only recognizable tool-managed badge/section content; leave handwritten content intact and warn about references the user may need to update.
3. A README update must link to the actual generated licence filename rather than assume a fixed `LICENCE.txt` path. `add` and `update` do not run `scan` automatically.
4. A README that is absent or already contains an equivalent licence section is a successful no-op. A write failure is reported distinctly from the licence-file result.
5. The CLI override must support both enabling and disabling the config default, so configuration can be overridden in either direction.

## Operational and quality requirements

- Release artifacts and executable names use `licencify` consistently across supported targets.
- CI runs `cargo audit` as a security check and exposes a local `just audit` recipe.
- Automated tests cover user-visible command outcomes and important boundaries: no false-positive detection, no accidental overwrite, manifest/file state changes, configuration precedence and path matching, cache/network fallback, proprietary behavior, and README idempotency.
  Include ambiguous primary filenames, dual licences, README/file conflicts, scan exclusions, local overrides, and failed promotion that leaves the old primary intact.
- Tests for stateful filesystem behavior are isolated; no test may leak global filesystem state into another test.
- The README and CLI help describe actual supported behavior and examples.

## Explicitly out of scope for this specification

- Adding SPDX headers to source files.
- Generating or validating full proprietary EULAs.
- Generating compound SPDX expressions such as `MIT OR Apache-2.0` or generating additional licence files automatically. `additional-licences` describes scan expectations, not how the licences combine legally.
- Adding arbitrary remote/custom registry protocols. Custom local template paths are covered; remote registry configuration appeared only in an early design proposal.
- Implementing features merely because an old plan lists them. This document defines target behavior; implementation status must be checked against the current code separately.

## Status and source notes

The plans index (`.hermes/plans/README.md`) records plans 001–005 as done:
release binary naming, Apache detection, `cargo audit`, command integration
tests, and the project README. The current README also documents SPDX-backed
list/search, text/HTML generation, layered configuration, templates, cache
commands, and schema generation.

The index does not record completion status for the older feature proposals on
global/project configuration, proprietary/no-file behavior, template alignment,
or README badge updating. Their presence in this specification reflects
proposed target behavior, not verified implementation status.

Primary source plans reviewed: `001`–`005`, `2026-06-18_180000-licencify-cli-design.md`, `2026-06-18_182000-spdx-integration-analysis.md`, `2026-06-18_183503-remaining-features.md`, `2026-06-19_0001-global-project-config-plan.md`, `2026-06-19_164138-proprietary-no-licence.md`, `2026-06-19_170000-licence-html-format-research.md`, and `2026-06-30_162000-readme-license-badge.md`.

Project and cache path conventions follow [PRJ Base Directory Specification v0.1](https://github.com/numtide/prj-spec/blob/main/PRJ_SPEC.md), which is marked unstable. Configuration uses one root shared file, one optional local override, and project-relative subdir rules rather than nested config files. Global user defaults remain optional.

## Open decisions for review

1. **SPDX refresh/versioning:** Confirm whether the embedded index is the only index used for validation, whether an `update-index` command is required, and which SPDX release/tag is authoritative for embedded text.
2. **Configuration writes:** Confirm whether `config set` should exist and which layer an explicit set command should write.
3. **Manifest update default:** The source plans conflict between opt-in per manifest and automatic updates. Confirm the default and whether a separate flag/config switch is required. Confirm npm `private = true` behavior for `UNLICENSED`.
4. **Proprietary behavior:** Confirm that `add proprietary` creates the notice by default, whether `--no-file` works for all licences, and whether the optional `remove` command belongs in scope.
5. **Custom templates vs remote registries:** Confirm that custom templates remain local paths only and that GitHub/custom remote registries are out of scope.

## Acceptance criteria for the consolidated product

- A user can discover valid SPDX IDs and metadata offline, then generate a supported common licence offline.
- An online user can generate licences beyond the embedded set, with fetched data cached and reusable.
- Add/update use the same source selection, rendering, primary-file targeting, config-write, prompting, and manifest rules. A format change never strands an old primary licence file, and promotion never deletes it on failure.
- Detection distinguishes supported licences without generic-phrase false positives and recognizes the proprietary notice; `scan` reports all conventional files and explicit README claims without reporting unknown text as an identified licence.
- Configuration resolves deterministically from optional global defaults through the root shared file, matching subdir rules, and the root local override to explicit CLI values; child config files are ignored with a warning. Subdirectory writes update an exact-path override by default, not project-wide defaults.
- `scan` compares identifiable evidence to effective primary and additional licence IDs, reports missing and unknown files as warnings, and fails on confirmed conflicts without modifying files.
- Optional README updates do not duplicate existing licence content and link to the output actually produced.
- Failures affecting files, manifests, configuration, or network lookup are visible and do not produce misleading success output.
- CLI docs and tests cover the confirmed behavior and its user-visible failure cases.
