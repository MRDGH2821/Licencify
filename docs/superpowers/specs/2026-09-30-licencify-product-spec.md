# Licencify Product Specification (Draft)

**Status:** Draft for maintainer review. This consolidates requirements and proposals from the plans in this directory; it is not an implementation plan.

## Purpose

Licencify is a command-line tool for selecting an open-source licence or a proprietary notice, rendering its text for a project, and optionally recording its status in project metadata. It uses the SPDX licence list as its open-source discovery and metadata source, with local templates and cached/fetched SPDX detail text for generation.

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

### Apply

1. `licencify apply [licence]...` writes the project's licence files. Each positional argument is an SPDX id or `proprietary`. No id uses the effective config and does not change the config file. One id sets `licence` and leaves `additional-licences` unchanged, except when promotion removes that id.
   Two or more ids set `licence` to the first and replace `additional-licences` with every id after the first. A repeated id is an error. `proprietary` cannot be combined with another id. If no id is given and the effective config has no licence, that is an error.
   Missing files for the effective `additional-licences` ids are generated. Existing extra files are not rewritten. An additional id removed from config leaves its file on disk.
2. `apply` accepts explicit author, company, email, year, and `txt`/`html`/`md` format options. An explicitly supplied CLI value overrides every configuration layer.
   Unset format falls back to `txt`; unset year uses the current year, company defaults to author, email is empty, README updates default to false, and the filename convention defaults to the detected locale. An unresolved author is an actionable error.
3. Supported template variables are `year`, `author`, `company`, `email`, and `date`. SPDX placeholders used in source text are rendered when recognized; unrecognized placeholders are not silently erased.
4. Plain-text output uses the template source order specified below. Custom templates use `<spdx-id>.tera` for text and `<spdx-id>.html.tera` for HTML. Markdown output converts rendered HTML to Markdown while preserving visible licence wording.
   If no HTML source exists for requested `md` or `html`, warn and write plain text with a `.txt` extension instead; fail if no plain-text source exists either.
5. When no primary file exists, `apply` writes one without asking. When one primary exists, `apply` locates it across the supported basename and format variants, confirms, and removes or renames the old path only after the replacement is ready. `--yes` skips that prompt. Scripted invocations must not wait for input. When several primary candidates exist, `apply` fails without changing files.
6. The config key `basename` is `licence`, `license`, or `copying`, matched without case. It is written as `LICENCE`, `LICENSE`, or `COPYING` plus the format extension. Unset, it follows the detected locale, which is `LICENCE` or `LICENSE`. `licence_file_name` and `licence_name` are rejected.
   Each additional licence uses a separate `<BASENAME>-<SPDX-ID>.<actual-format>` file, such as `LICENCE-Apache-2.0.txt` or `COPYING-MIT.txt`. A warned HTML/Markdown fallback uses `.txt`.
   Existing additional files are never rewritten, including when their extensions differ from the selected format. Validate IDs and output paths and resolve every required template before modifying files; failures in additional generation must not be reported as full command success. `scan` separately reports unidentified or conflicting existing content.
7. If the requested primary ID is in `additional-licences`, reject the command unless `--permit-promotion` is supplied. Promotion moves exactly one existing matching additional file into the primary position, removes the old primary, removes the promoted ID from the effective additional list, and records it as primary.
   Do not silently relabel a source format, guess among multiple source files, or delete the old primary before the move and config change can succeed. `--permit-promotion` authorizes this transition; `--yes` separately controls prompts.
8. `apply` reports every file written, the selected primary and additional licences, and any manifest or readme updates or skipped operations. With `--verbose`, report each resolved setting's source (CLI, config file and matching rule, or fallback) without printing its value.

### Detection

1. `licencify detect` checks recognized licence filenames in the current project and attempts to identify supported licence text using distinctive signatures.
2. A single generic phrase such as "version 2.0" is insufficient to identify a licence. Apache-2.0 detection requires both an Apache-specific and version-specific signature.
3. The detector recognizes the built-in licence families documented by the CLI, including proprietary notices as `proprietary`.
4. Missing files, unreadable files, and unrecognized contents are reported as errors or no-match outcomes; they do not terminate the process through an internal process exit.

### Project scan

1. `licencify scan` recursively discovers conventional `LICENCE`, `LICENSE`, and `COPYING` files (including `.txt`, `.md`, and `.html` variants and ID-suffixed additional files) beneath the selected project root. Unlike `detect`, it reports every result rather than stopping at the first.
   It also examines READMEs for explicit licence declarations, badges, and licence-file links, not incidental mentions in examples.
2. Skip Git-ignored directories and project-relative directories listed in project configuration under `[scan] exclude = ["vendor"]`. Exclusion paths cannot escape the project root; do not follow directory symlinks outside it. If an exclusion overlaps a configured `[[subdirs]]` path, warn that the configuration is contradictory and scan that subdir anyway.
3. For each result, show its path and evidence level: identified from distinctive text, claimed by a filename or README declaration, or unidentified. An unrecognized file is reported as unknown, not assigned a guessed SPDX ID. Report every file in a multi-licence directory and conflicts between confidently identified text and explicit file/README claims.
4. Discovery works without any configuration. If configuration exists, compare findings against effective `licence` and `additional-licences` at the root, configured subdir paths, and directories with discovered evidence. An optional explicit scan ID overrides the expected primary ID, not the additional list. Traversal does not require every visited directory to have a licence file.
5. A confirmed conflicting ID or unreadable file causes a nonzero result; unknown identity, missing expected licence file or directory, and contradictory exclusions cause warnings. A missing configured path is not silently reported as matching. The scan is read-only and does not run automatically after `apply`.

### Configuration

1. Select the project root from an absolute `PRJ_ROOT` when set, otherwise the Git worktree root, otherwise the highest ancestor containing a project config, otherwise the current directory. Reject an invocation whose current directory lies outside an explicit `PRJ_ROOT`. `PRJ_CONFIG_HOME`, when set, replaces `$PRJ_ROOT/.config` as the project config directory.
2. Load global defaults from `$XDG_CONFIG_HOME/licencify/config.toml` (or the platform equivalent), then the root shared config at `${PRJ_CONFIG_HOME:-$PRJ_ROOT/.config}/licencify/config.toml`, then its optional `config.local.toml`. The local file is intended to remain uncommitted. Legacy root-level config filenames are not loaded.
3. Only root project config files are loaded. When invoked from a child directory containing a Licencify config file on the path from the root to the current directory, warn that it was ignored and recommend merging it into the root shared config. Do not search the whole project tree for child configs.
4. `[default]` supports optional `author`, `company`, `email`, `year`, `licence` (one SPDX ID or the literal `proprietary`), `additional-licences` (an array of other SPDX IDs), `format`, `update-readme`, and `basename` (`licence`, `license`, or `copying`).
   A key that contains more than one word uses hyphens. An underscore spelling of the same key is rejected. `licence_file_name` and `licence_name` are rejected.
   Project files may additionally contain `[[subdirs]]` entries with a required `path` and optional overrides for any of these fields; global `[[subdirs]]` entries are invalid.
   Subdir paths are normalized relative to the project root, cannot escape it, and match only complete path components. A later `additional-licences` array replaces an earlier one; `[]` clears inherited additional IDs.
   Reject any effective configuration that mixes `proprietary` with open-source additional IDs, whether proprietary is primary or appears in the additional list.
5. Resolve fields in this order, lowest to highest: global defaults; shared project defaults; matching shared subdirs from shallowest to deepest; local project defaults; matching local subdirs from shallowest to deepest; explicit CLI options.
   Later sources override only fields they specify. Entries for the same normalized path in shared and local files contribute only their explicit fields at their respective precedence positions. A local default therefore overrides a shared subdir value; a local subdir can override it again.
6. If a required licence or author remains unresolved after CLI and configuration lookup, report an actionable error. Missing optional fields use the fallbacks specified under Apply; neither a global config file nor every optional field is mandatory. Invalid TOML and invalid values of known fields are errors. Warn with file and key for unknown config options and ignore those options.
7. `init` creates only the root shared config at `${PRJ_CONFIG_HOME:-$PRJ_ROOT/.config}/licencify/config.toml` without overwriting it; it does not create the global or local file. The new file starts with `#:schema https://raw.githubusercontent.com/MRDGH2821/Licencify/main/schema/config.json`. `config get` prints one key. `config list` identifies loaded files and effective values.
   A cargo example writes `schema/config.json`. `cargo test` fails if that file drifts. The binary has no schema command.
8. When `apply` is given one or more ids, it records the chosen primary id, author, and actual format in configuration, preserving unrelated values and comments. Two or more ids also replace `additional-licences`. No id does not write config.
   From the project root, write the shared defaults. From a subdirectory, write an exact-path `[[subdirs]]` entry. `--root` writes the shared defaults from either place. There is no destination prompt.
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
2. All provider and CLI cache commands use one user-level SPDX detail cache at `${XDG_CACHE_HOME:-$HOME/.cache}/licencify/SPDX-Cache/` (or the platform user cache directory elsewhere). No project cache, `PRJ_CACHE_HOME`, or `PRJ_ID` affects it; existing `api/` cache data is not migrated.
3. On an SPDX detail cache hit, use that detail without a network request; if its HTML field is absent, continue to bundled HTML or the plain-text fallback rather than refetching the same detail. On a cache miss, fetch the detail and cache the successful response. Cached detail remains available until `cache clear`; this specification defines no time-based expiry.
4. A cache write failure does not discard a successfully fetched response. A network failure may fall back to a built-in template where available; otherwise report an actionable error.
5. `licencify cache` with no subcommand reports the resolved SPDX cache directory, the number of cached files, and the size in bytes. `cache clear` removes only the Licencify `SPDX-Cache/` directory; clearing the cache must not delete configuration or other user data.
   `cache fetch` with no id downloads every uncached detail JSON. `cache fetch <id>` downloads that one id and leaves a cached copy in place. `cache update` refreshes `licenses.json` only and does not download bodies.
6. Built-in templates provide an offline generation path for the supported common licences.
7. For each requested format, search project templates, global templates, cached SPDX detail, fetched SPDX detail on a cache miss, then bundled templates, in that order. HTML and Markdown use the SPDX detail's `licenseTextHtml` only when present; plain text uses `licenseText`.
   A custom template wins even when SPDX detail is already cached. A failed fetch may still use a bundled template. Missing HTML follows the warned `.txt` fallback above. A template that exists but cannot be read or rendered fails visibly rather than silently selecting another source.

### Manifest integration

1. When manifest updates are enabled, Licencify detects supported manifests and writes the selected SPDX identifier using the ecosystem's supported representation. If the effective `additional-licences` array is nonempty, skip manifest updates and warn: the array does not specify whether the IDs combine with `OR` or `AND`.
2. Initial target manifests are `Cargo.toml`, `package.json`, and `pyproject.toml`. Existing formatting/comments should be preserved where the file format permits.
3. An unsupported or malformed manifest is reported with its path. Failure to update one manifest must not be reported as success for that manifest.
4. Proprietary manifest updates must not write `proprietary` as an SPDX expression. For Cargo, set `publish = false`, remove a stale SPDX `license`, and set `license-file` to the generated primary notice. For npm, set `private = true` and `license = "SEE LICENSE IN <filename>"`.
   For Python, remove a stale `project.license`, include the notice with `license-files` where supported, and warn that private publication must be controlled separately.
   Switching back to an SPDX licence does not automatically clear Cargo `publish = false` or npm `private = true`; warn instead.
5. `--no-file` skips writing every licence file, including additional files, while retaining explicitly enabled manifest updates. For proprietary, manifest references to the notice require an existing primary notice; if absent, fail the affected manifest update rather than point to a nonexistent file.

### Proprietary declaration

1. `proprietary` is a Licencify configuration and CLI special value, not an SPDX licence-list entry. Do not use `UNLICENSED` as its Licencify config value or write `proprietary` to SPDX manifest fields.
2. `licencify add proprietary` generates a short copyright / "All rights reserved" notice by default, not a full EULA or legal document.
3. For a proprietary notice, resolve project `proprietary.tera` or `proprietary.html.tera`, then the global equivalent, then the bundled equivalent. Do not fetch SPDX detail or consult the SPDX cache for proprietary.
4. Deleting a licence file is out of scope. There is no `remove` command.

### README update

1. README modification is opt-in by CLI or configuration; the default is no change.
2. When enabled, Licencify locates a supported Markdown README, adds a licence badge and licence section only when an equivalent one is not already present, and leaves non-Markdown README formats unchanged. On a licence change, replace only recognizable tool-managed badge/section content; leave handwritten content intact and warn about references the user may need to update.
3. A README update must link to the actual generated licence filename rather than assume a fixed `LICENCE.txt` path. `apply` does not run `scan` automatically.
4. A README that is absent or already contains an equivalent licence section is a successful no-op. A write failure is reported distinctly from the licence-file result.
5. The CLI override must support both enabling and disabling the config default, so configuration can be overridden in either direction.

## Operational and quality requirements

- Release artifacts and executable names use `licencify` consistently across supported targets.
- CI runs `cargo audit` as a security check and exposes a local `mise run audit` task in `mise.toml` (to be added during implementation).
- Automated tests cover user-visible command outcomes and important boundaries: no false-positive detection, no accidental overwrite, manifest/file state changes, configuration precedence and path matching, cache/network fallback, proprietary behavior, and README idempotency.
  Include ambiguous primary filenames, dual licences, README/file conflicts, scan exclusions, local overrides, and failed promotion that leaves the old primary intact.
- Tests for stateful filesystem behavior are isolated; no test may leak global filesystem state into another test.
- The README and CLI help describe actual supported behavior and examples.

## Explicitly out of scope for this specification

- Adding SPDX headers to source files.
- Generating or validating full proprietary EULAs.
- Generating compound SPDX expressions such as `MIT OR Apache-2.0`. `additional-licences` generates missing separate files and describes scan expectations, but does not define how the licences combine legally.
- Adding arbitrary remote/custom registry protocols. Custom local template paths are covered; remote registry configuration appeared only in an early design proposal.
- Implementing features merely because an old plan lists them. This document defines target behavior; implementation status must be checked against the current code separately.

## Status and source notes

Those plans are now ADRs in `docs/adr/`. This specification remains the target behavior. Implementation status still has to be checked against the current code.

Project configuration paths follow [PRJ Base Directory Specification v0.1](https://github.com/numtide/prj-spec/blob/main/PRJ_SPEC.md), which is marked unstable; the SPDX cache instead uses a single user-level directory. Configuration uses one root shared file, one optional local override, and project-relative subdir rules rather than nested config files. Global user defaults remain optional.

## Decided

1. **SPDX index:** `cache update` caches `licenses.json`. That cached file is the validation, list, and search index when it exists. The embedded 3.28.0 list is the fallback. The command does not download licence bodies.
2. **Licence wording:** Each `{id}.json` from `fetch_detail` or `cache fetch` is the template body. `cache fetch` with no id downloads every uncached detail. `cache fetch <id>` downloads that one id and does not refetch a cached copy. A local custom template overrides the detail JSON. A bundled template is used only when that JSON is missing or the fetch fails.
3. **Config writes:** `init` is a root command. `config get <key>` prints one field. `config list` prints the effective config. `config set <option>=<value>` sets any `[default]` field, including the manifest-update switch. It writes shared defaults from the project root, or an exact-path `[[subdirs]]` entry from a subdirectory.
   A cargo example writes `schema/config.json`. `cargo test` fails if that file drifts. The schema URL is <https://raw.githubusercontent.com/MRDGH2821/Licencify/main/schema/config.json>. The binary has no schema command.
   The verb survey is in [`.agents/research/cli-subcommand-verbs.md`](../../../.agents/research/cli-subcommand-verbs.md). Schema generation is in [`.agents/research/config-schema-generation.md`](../../../.agents/research/config-schema-generation.md).
   The command split is [ADR 0013](../../adr/0013-config-get-list-set.md), [ADR 0014](../../adr/0014-cache-update-and-fetch.md), and [ADR 0016](../../adr/0016-project-licences-are-applied.md). The schema writer is [ADR 0015](../../adr/0015-config-schema-is-a-cargo-example.md).
4. **Manifest updates:** Off unless one switch turns them on for Cargo, npm, and Python together.
5. **Templates:** Custom templates are local files only. Remote registries are out of scope.
6. **Removal:** Deleting a licence file is out of scope.
7. **Proprietary manifests:** A proprietary manifest update keeps today's rules: npm `private` plus `SEE LICENSE IN <file>`, Cargo `publish = false`, and no `UNLICENSED` string. It does not use the Unlicense text.
8. **Mixing:** `proprietary` cannot be combined with `Unlicense` or any other open licence. `Unlicense` may be primary or additional, like any other SPDX id.
9. **Unlicense spelling:** `unlicence`, `unlicense`, and `Unlicense` select the same licence, in any case. Config and manifests store `Unlicense`.
10. **Detection labels:** Unlicense text is reported as `Unlicense`. A proprietary notice is reported as `proprietary`.
11. **`UNLICENSED` input:** `UNLICENSED` is an unknown id. Only `proprietary` selects the proprietary notice. Removing the current code alias is deferred.
12. **Manifest-update switch:** Config key `update-manifest` defaults off. Underscore spellings of config keys are rejected. CLI flags are `--update-manifest` and `--no-update-manifest`, on `apply`, and they conflict with each other.
13. **Apply:** `licencify apply` replaces `add` and `update`. No id uses the effective config and does not write config. One id sets `licence` only. Two or more ids set the first as `licence` and replace `additional-licences` with the rest. `--root` writes the shared section. Otherwise the current directory chooses the shared section or an exact-path `[[subdirs]]` entry.
14. **Primary filename:** Config key `basename` is `licence`, `license`, or `copying`. The written file is `LICENCE`, `LICENSE`, or `COPYING` plus the format extension. [ADR 0017](../../adr/0017-primary-file-basename.md).

The naming split in items 7–11 is recorded in [ADR 0001](../../adr/0001-unlicense-and-proprietary.md).

## Acceptance criteria for the consolidated product

- A user can discover valid SPDX IDs and metadata offline, then generate a supported common licence offline.
- An online user can generate licences beyond the embedded set, with fetched data cached and reusable.
- Add/update use the same source selection, rendering, primary-file targeting, additional-file generation, config-write, prompting, and manifest rules. A format change never strands an old primary licence file, and promotion never deletes it on failure.
- Detection distinguishes supported licences without generic-phrase false positives and recognizes the proprietary notice; `scan` reports all conventional files and explicit README claims without reporting unknown text as an identified licence.
- Configuration resolves deterministically from optional global defaults through the root shared file, matching subdir rules, and the root local override to explicit CLI values; child config files are ignored with a warning. Subdirectory writes update an exact-path override by default, not project-wide defaults.
- `scan` compares identifiable evidence to effective primary and additional licence IDs, reports missing and unknown files as warnings, and fails on confirmed conflicts without modifying files.
- Each missing configured additional licence gets a separate correctly named file on add/update; existing extra files remain untouched, `--no-file` skips all licence files, and a failure to generate any required extra is reported.
- Optional README updates do not duplicate existing licence content and link to the output actually produced.
- Failures affecting files, manifests, configuration, or network lookup are visible and do not produce misleading success output.
- CLI docs and tests cover the confirmed behavior and its user-visible failure cases.
