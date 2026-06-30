# Plan 005: Write project README

> **Executor instructions**: Follow this plan step by step. Run every
> verification command and confirm the expected result before moving to the
> next step. If anything in the "STOP conditions" section occurs, stop and
> report — do not improvise. When done, update the status row for this plan
> in `plans/README.md`.
>
> **Drift check (run first)**: `git diff --stat 90eb384..HEAD -- README.md`
> If the file was modified since this plan was written, compare the "Current
> state" against the live file; on a mismatch, consider this plan superseded.

## Status

- **Priority**: P2
- **Effort**: S
- **Risk**: LOW
- **Depends on**: none
- **Category**: docs
- **Planned at**: commit `90eb384`, 2026-06-30

## Why this matters

The README is 7 lines and contains only a copier badge and a license link.
Someone landing on the repo sees no explanation of what the tool does, how
to install it, or how to use it. This is the first impression for users and
contributors. Writing a proper README reduces support questions, makes the
project discoverable, and establishes project credibility.

## Current state

```markdown
# Licencify

[![Copier](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/copier-org/copier/refs/heads/master/img/badge/black-badge.json)](https://github.com/copier-org/copier)

## Licence

See [LICENCE](./LICENSE)
```

The badge references "Copier" (the project scaffold generator) — it's a
tooling detail, not useful for end-users.

## Commands you will need

| Purpose | Command                                   | Expected on success |
| ------- | ----------------------------------------- | ------------------- |
| Build   | `cargo build`                             | exit 0              |
| Help    | `./target/debug/licencify --help`         | prints usage        |
| List    | `./target/debug/licencify list --limit 5` | lists 5 licenses    |

## Scope

**In scope**:

- `README.md` — full rewrite

**Out of scope**:

- Any other documentation files (AGENTS.md, CHANGELOG.md, etc.)
- Any source code changes

## Git workflow

- Branch: `advisor/005-write-readme`
- Commit message: `docs: add project README with install and usage`
- Do NOT push or open a PR unless instructed.

## Steps

### Step 1: Capture current CLI help text

Run the following and capture the output for inclusion in the README:

```
./target/debug/licencify --help
```

Also capture:

```
./target/debug/licencify add --help
./target/debug/licencify list --help
```

### Step 2: Write the README

Replace `README.md` with a comprehensive README that includes the following
sections in order. Below is the content to write — adapt the help text
sections from the actual CLI output if the arguments have changed.

````markdown
# Licencify

A CLI tool to add open-source licenses to projects. Fetches license text
from the [SPDX License List](https://spdx.org/licenses/), caches it
locally, and writes it to your project as `LICENCE.txt` / `LICENSE.txt`
or HTML. Also updates your project manifest (`Cargo.toml`, `package.json`,
`pyproject.toml`) with the SPDX identifier.

## Install

### From source

```bash
git clone <repo-url>
cd licencify
cargo build --release
./target/release/licencify --help
```
````

### With cargo

```bash
cargo install --path .
licencify --help
```

## Quick start

```bash
# Add the MIT license
licencify add MIT

# Add a license with custom author and year
licencify add Apache-2.0 --author "Jane Doe" --year 2025

# List all available licenses
licencify list

# Search for a license
licencify search bsd

# Detect which license is in the current project
licencify detect
```

## Usage

```
<insert output of `licencify --help` here>
```

### Adding a license

```
<insert output of `licencify add --help` here>
```

The `add` command walks through a confirmation prompt before writing.
Use `-Y` to skip prompts:

```bash
licencify add MIT -Y
```

### License detection

`licencify detect` scans the current directory for common license filenames
(`LICENSE`, `LICENCE`, `COPYING`, etc.) and identifies the license from its
text content using keyword matching.

### Configuration

Licencify supports global (`~/.config/licencify/config.toml`) and
per-project (`.licencify.toml`) configuration. Initialize with:

```bash
licencify config init
```

Configurable defaults include author name, company, email, default license
ID, output format (txt/html), and the `LICENCE` vs `LICENSE` naming
convention.

Per-subdirectory overrides are supported for monorepos. See
`licencify config init` output for details.

### License templates

Built-in templates exist for 14 common licenses (MIT, Apache-2.0,
GPL-2.0/3.0, AGPL-3.0, LGPL-3.0, BSD-2/3-Clause, MPL-2.0, ISC, Unlicense,
CC0-1.0, WTFPL, Proprietary) in both plain text and HTML format. Templates
are rendered with Tera and support `{{ year }}`, `{{ author }}`,
`{{ company }}`, `{{ email }}`, `{{ date }}` placeholders.

For licenses without built-in templates, text is fetched from the SPDX API
and cached locally (`~/.cache/licencify/api/`).

### Generating config schema

```bash
licencify schema
```

Writes `licencify-schema.json` to the current directory for IDE autocompletion.

## License

See [LICENCE](./LICENSE)

```

**Important**: Replace `<insert output of ...>` with the actual help text
from Step 1, formatted as code blocks.

**Important**: Keep the original `[![Copier]...` badge line or remove it —
it's a tooling detail that doesn't serve the README's audience. If keeping
it, move it to the bottom or to a "Development" section.

**Repo conventions to match**:
- British English spelling in README body (licence, not license), matching
  the project's British English conventions and the `AGENTS.md` guidance.
- The license file is `./LICENSE` (en-US spelling used as the actual filename
  per convention).
- Markdown formatting: wrap at reasonable line lengths. No trailing whitespace.

**Verify**:
```

cargo build 2>&1 | tail -3

```
Expected: shows no errors (doc build doesn't fail on README, but confirms
the repo is in a valid state).

## Test plan

No tests to write — this is a documentation change. Verification:
- The README renders correctly on GitHub (markdown preview).
- The install commands are syntactically correct and match the project's
  actual build system.

## Done criteria

- [ ] `README.md` contains at least: project description, install instructions,
      quick-start example, usage section with help output, config section,
      license section
- [ ] All CLI command examples in the README match the actual CLI (verify
      by running the exact commands)
- [ ] No source files modified (`git status` shows only `README.md`)
- [ ] `plans/README.md` status row updated

## STOP conditions

Stop and report back if:

- The CLI help output contains commands or flags not described in the plan —
  include them in the README (the plan is a template, not a complete reference).
- The project has a different install method documented elsewhere that
  conflicts with what's written here.

## Maintenance notes

- Keep the help-text examples in sync with CLI changes — they're the most
  likely section to drift.
- When new commands are added, add a section in the README.
```
