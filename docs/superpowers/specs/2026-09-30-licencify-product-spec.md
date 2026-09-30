# Licencify Product Specification (Draft)

**Status:** Draft for maintainer review. This consolidates requirements and proposals from the plans in this directory; it is not an implementation plan.

## Purpose

Licencify is a command-line tool for selecting an open-source licence, rendering its text for a project, and optionally recording the licence in project metadata. It uses the SPDX licence list as its discovery and metadata source, with local templates and cached/fetched SPDX detail text for generation.

## Goals

- Make licence discovery and metadata lookup available offline.
- Generate plain-text or HTML licence files with user/project-specific copyright details.
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

1. `licencify add <ID>` generates a licence for the current project; `licencify update <ID>` replaces the existing licence using the same rendering and metadata rules.
2. Both commands support author, company, email, year, and `txt`/`html` format options. Defaults may come from configuration; author resolution may use Git configuration. Missing required values produce an actionable error.
3. Supported template variables are `year`, `author`, `company`, `email`, and `date`. SPDX placeholders used in source text are rendered when recognized; unrecognized placeholders are not silently erased.
4. Plain-text output is based on SPDX-authoritative text where available. Built-in templates remain available without network access. Custom templates use the configured template paths and the documented `<spdx-id>.tera` / `<spdx-id>.html.tera` naming convention.
5. Before replacing an existing licence file, the command follows an explicit confirmation policy. A non-interactive/scripted invocation must not block waiting for input; `--yes` uses defaults and skips prompts.
6. Output naming respects the configured `LICENCE`/`LICENSE` convention and selected format.
7. The commands report which file was written, the selected licence, and any manifest/readme updates or skipped operations.

### Detection

1. `licencify detect` checks recognized licence filenames in the current project and attempts to identify supported licence text using distinctive signatures.
2. A single generic phrase such as “version 2.0” is insufficient to identify a licence. Apache-2.0 detection requires both an Apache-specific and version-specific signature.
3. The detector recognizes the built-in licence families documented by the CLI, including proprietary notices as `UNLICENSED`.
4. Missing files, unreadable files, and unrecognized contents are reported as errors or no-match outcomes; they do not terminate the process through an internal process exit.

### Configuration

1. Configuration supports global user defaults, project overrides, and project subdirectory overrides.
2. Precedence is: global defaults, then project values, then the matching subdirectory values, then explicit CLI values. An unset value at a higher level does not erase a lower-level value.
3. The project configuration is discovered from the working directory or an ancestor project root. Subdirectory overrides are matched against paths relative to that root, using the most-specific path match and respecting path-component boundaries.
4. Configuration covers licence ID, author, format, year, licence filename convention, template paths, and the optional README-update default.
5. `config init` creates the intended config without overwriting an existing file. `config show` identifies the configuration sources and effective values. `config set` writes to the agreed configuration layer. `schema` writes a JSON schema to the requested path.
6. Invalid or unreadable configuration must be reported; it must not silently become empty defaults.

### Templates, cache, and network

1. The embedded SPDX index supports validation, listing, searching, and metadata without network access.
2. The SPDX detail endpoint supplies full licence text and, where selected, HTML text. Successful detail responses are cached in the user's Licencify cache; `cache info` reports its location/size and `cache clear` removes cached template data.
3. Built-in templates provide an offline generation path for the supported common licences. A network outage must not prevent generating one of those licences.
4. Template sources and their precedence must be deterministic. The intended source categories are configured custom templates, embedded templates, cached SPDX detail, and fetched SPDX detail; their exact precedence is a review decision (see Open decisions).
5. Network or cache failures are reported with context. A cache write failure alone should not discard an otherwise successfully fetched licence.

### Manifest integration

1. When manifest updates are enabled, Licencify detects supported manifests and writes the selected SPDX identifier using the ecosystem's supported representation.
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
2. When enabled, Licencify locates a supported Markdown README, adds a licence badge and licence section only when an equivalent one is not already present, and leaves non-Markdown README formats unchanged.
3. A README update must link to the actual generated licence filename rather than assume a fixed `LICENCE.txt` path.
4. A README that is absent or already contains a licence section is a successful no-op. A write failure is reported distinctly from the licence-file result.
5. The CLI override must support both enabling and disabling the config default, so configuration can be overridden in either direction.

## Operational and quality requirements

- Release artifacts and executable names use `licencify` consistently across supported targets.
- CI runs `cargo audit` as a security check and exposes a local `just audit` recipe.
- Automated tests cover user-visible command outcomes and important boundaries: no false-positive detection, no accidental overwrite, manifest/file state changes, configuration precedence and path matching, cache/network fallback, proprietary behavior, and README idempotency.
- Tests for stateful filesystem behavior are isolated; no test may leak global filesystem state into another test.
- The README and CLI help describe actual supported behavior and examples.

## Explicitly out of scope for this specification

- Adding SPDX headers to source files.
- Generating or validating full proprietary EULAs.
- Supporting compound SPDX expressions such as `MIT OR Apache-2.0` as a generated licence.
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

## Open decisions for review

1. **Template source precedence:** The README says custom templates are checked before built-ins and SPDX is a fallback. Plans additionally specify a disk cache and disagree on whether built-ins or fetched SPDX content take precedence. Confirm one order, including how offline mode behaves.
2. **SPDX refresh/versioning:** Confirm whether the embedded index is the only index used for validation, whether an `update-index` command is required, and which SPDX release/tag is authoritative for embedded text.
3. **Configuration filenames and subdirectory schema:** The README documents `.licencify.toml` or `licencify.toml` and array-form `[[subdirs]]` entries. The older plan uses only `licencify.toml` and a keyed `[subdirs."path"]` table. Select the canonical form and decide whether the other form/path remains supported.
4. **Configuration writes:** Confirm whether `config init` creates project config by default, whether a `--global` option is needed, and whether `config set` should exist and always write globally.
5. **Manifest update default:** The source plans conflict between opt-in per manifest and automatic updates. Confirm the default and whether a separate flag/config switch is required. Confirm npm `private = true` behavior for `UNLICENSED`.
6. **HTML output contract:** One proposal uses SPDX `licenseTextHtml`; another builds self-contained HTML documents around plain text in `<pre>`. Confirm output source/structure and whether it must include a document title. Also confirm whether exact plain-text line wrapping to a pinned SPDX release is required.
7. **Proprietary behavior:** Confirm that `add proprietary` creates the notice by default, whether `--no-file` works for all licences, and whether the optional `remove` command belongs in scope.
8. **README update CLI semantics:** Confirm explicit enable/disable flag names and precedence. The plan's boolean handling does not clearly represent “unset” versus “explicit false,” which is required to override a true config default.
9. **Custom templates vs remote registries:** Confirm that custom templates remain local paths only and that GitHub/custom remote registries are out of scope.

## Acceptance criteria for the consolidated product

- A user can discover valid SPDX IDs and metadata offline, then generate a supported common licence offline.
- An online user can generate licences beyond the embedded set, with fetched data cached and reusable.
- Add/update use the same source selection, rendering, naming, prompting, and manifest rules.
- Detection distinguishes supported licences without generic-phrase false positives and recognizes the proprietary notice.
- Configuration sources have documented, deterministic precedence and subdirectory matching does not confuse sibling path prefixes.
- Optional README updates do not duplicate existing licence content and link to the output actually produced.
- Failures affecting files, manifests, configuration, or network lookup are visible and do not produce misleading success output.
- CLI docs and tests cover the confirmed behavior and its user-visible failure cases.
