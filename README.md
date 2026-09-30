# Licencify

[![Copier](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/copier-org/copier/refs/heads/master/img/badge/black-badge.json)](https://github.com/copier-org/copier)

A CLI tool to add open-source licences to your projects. Fetches licence templates from the SPDX index, renders them with your details, and writes them to `LICENCE` or `LICENSE`.

## Install

### From source

```bash
git clone https://github.com/MRDGH2821/Licencify
cd Licencify
cargo build --release
# Binary at ./target/release/licencify
```

### With Cargo

```bash
cargo install --git https://github.com/MRDGH2821/Licencify
```

## Development

Install [mise](https://mise.jdx.dev/), then run `mise install` in the repository to install the pinned Rust toolchain, `cargo-audit`, and `mr-boxington`.
Use `mise run audit` from the repository root to run `cargo audit`; CI continues to run the configured audit job.
Use `mise exec -- cargo build` or `mise exec -- cargo test` to build or test with project-scoped `mbx` wrapping; its optional `scheduler.tests` setting remains disabled.
To undo this setup, remove the Rust, `cargo:cargo-audit`, and `mr-boxington` entries from `mise.toml` and the corresponding entries from `mise.lock` if generated; removing the Rust entry also removes its `mr_boxington` option.

## Quick start

```bash
# Add an MIT licence (author auto-detected from git config)
licencify add MIT

# Add Apache-2.0 with a custom author, skip all prompts
licencify add Apache-2.0 --author "Jane Doe" --year 2025 -Y

# List available licences
licencify list

# Search for a licence by name or ID
licencify search bsd

# Detect the current project's licence from existing LICENCE file
licencify detect
```

## Usage

```text
Add open-source licenses to projects

Usage: licencify <COMMAND>

Commands:
  add     Add a license to the current project
  list    List available licenses
  search  Search available licenses by name or ID
  detect  Detect the current project's license
  update  Change the project's license
  cache   Manage local template cache
  config  Manage configuration
  schema  Generate JSON schema for config file
  help    Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Adding a licence

```text
Add a license to the current project

Usage: licencify add [OPTIONS] <SPDX>

Arguments:
  <SPDX>
          SPDX license identifier (e.g., MIT, Apache-2.0, proprietary)

Options:
  -a, --author <AUTHOR>
          Copyright holder name (default: git config user.name)

      --company <COMPANY>
          Company name (defaults to author)

      --email <EMAIL>
          Contact email address

  -y, --year <YEAR>
          Copyright year (default: current year)

  -f, --format <FORMAT>
          Output format: txt (default), html, or md

          Possible values:
          - txt:  Plain text (licenseText)
          - html: HTML (licenseTextHtml)
          - md:   Markdown converted from licenseTextHtml

          [default: txt]

  -Y, --yes
          Skip all prompts and use defaults

  -h, --help
          Print help (see a summary with '-h')
```

The `-Y` (or `--yes`) flag is useful for scripting — it skips all confirmation prompts and uses defaults for any unset values.

## Listing licences

```text
List available licenses

Usage: licencify list [OPTIONS]

Options:
      --osi-only       Show only OSI-approved licenses
      --fsf-only       Show only FSF Libre licenses
  -l, --limit <LIMIT>  Paginate results (max licenses to show)
  -h, --help           Print help
```

## Searching licences

```text
Search available licenses by name or ID

Usage: licencify search [OPTIONS] <QUERY>

Arguments:
  <QUERY>  Search query (matches name or license ID)

Options:
      --osi-only  Show only OSI-approved licenses
      --fsf-only  Show only FSF Libre licenses
  -h, --help      Print help
```

## Changing a licence

```text
Change the project's license

Usage: licencify update [OPTIONS] <SPDX>

Arguments:
  <SPDX>
          SPDX license identifier to change to

Options:
  -a, --author <AUTHOR>
          Copyright holder name

      --company <COMPANY>
          Company name (defaults to author)

      --email <EMAIL>
          Contact email address

  -y, --year <YEAR>
          Copyright year

  -f, --format <FORMAT>
          Output format: txt (default), html, or md

          Possible values:
          - txt:  Plain text (licenseText)
          - html: HTML (licenseTextHtml)
          - md:   Markdown converted from licenseTextHtml

          [default: txt]

  -h, --help
          Print help (see a summary with '-h')
```

## Licence detection

The `detect` command reads your project's `LICENCE` or `LICENSE` file and identifies the licence using keyword matching. It recognises MIT, Apache-2.0, GPL-3.0-only, GPL-2.0-only, LGPL-3.0-only, MPL-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unlicense, and proprietary (`UNLICENSED`).

```text
Detect the current project's license

Usage: licencify detect

Options:
  -h, --help  Print help
```

## Project licence scan

`licencify scan` reads conventional licence files and explicit README declarations throughout the Git project without changing files. It reports identified text, filename or README claims, and unknown content separately. Confirmed conflicts and unreadable files fail; missing expected licences or directories warn. Git-ignored paths are skipped.

```bash
licencify scan
licencify scan MIT # Override the expected primary ID, not additional licences.
```

Project config may exclude directories; a configured `[[subdirs]]` rule overlapping an exclusion remains scanned and produces a warning:

```toml
[scan]
exclude = ["vendor", "build"]
```

## Configuration

Licencify supports layered configuration: global, project-level, and per-directory overrides.

Initialise a default config:

```bash
licencify config init
```

```text
Manage configuration

Usage: licencify config <COMMAND>

Commands:
  init  Create default config file
  show  Show current configuration
  help  Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### Config locations

- **Global defaults**: `$XDG_CONFIG_HOME/licencify/config.toml` (or the platform config directory)
- **Shared project config**: `${PRJ_CONFIG_HOME:-$PRJ_ROOT/.config}/licencify/config.toml`
- **Local project overrides**: the adjacent `config.local.toml`

`PRJ_ROOT` may select an absolute root containing the current directory. Otherwise Licencify uses the Git worktree root, the highest ancestor with a shared config, or the current directory. Root-level `.licencify.toml` and `licencify.toml` are not loaded. `config init` creates only the shared project config without overwriting it.

Global defaults are overridden, field by field, by shared project defaults, matching shared `[[subdirs]]` entries (shallow to deep), local defaults, matching local entries (shallow to deep), then CLI options. An empty `additional-licences` array clears inherited additional IDs.

### Subdirectory overrides

Define rules in either root project config, not in child config files:

```toml
[default]
author = "Jane Doe"
licence = "MIT"

[[subdirs]]
path = "docs"
licence = "CC0-1.0"

[[subdirs]]
path = "docs/api"
additional-licences = []
```

Paths are relative to the project root; paths that escape it are rejected. Child config files on the path to the current directory are ignored with a warning.

`add` and `update` preserve comments and unrelated settings when recording the selected licence, author, and format. From a subdirectory, interactive use asks whether to update shared defaults or the exact subdirectory rule; `--yes` and noninteractive use select the exact rule.

Set `--config-target shared` or `--config-target subdir` explicitly. If a local override masks the chosen fields, the winning local entry is updated instead. `--verbose` reports setting sources without printing their values.

## Licence templates

Licencify ships with 14 built-in template pairs (plain text + HTML); Markdown is converted from HTML:

| SPDX ID         | Licence                      |
| --------------- | ---------------------------- |
| `mit`           | MIT License                  |
| `apache-2.0`    | Apache License 2.0           |
| `gpl-3.0-only`  | GNU GPL v3                   |
| `gpl-2.0-only`  | GNU GPL v2                   |
| `agpl-3.0-only` | GNU AGPL v3                  |
| `lgpl-3.0-only` | GNU LGPL v3                  |
| `bsd-2-clause`  | BSD 2-Clause                 |
| `bsd-3-clause`  | BSD 3-Clause                 |
| `mpl-2.0`       | Mozilla Public License 2.0   |
| `unlicense`     | The Unlicense                |
| `cc0-1.0`       | Creative Commons Zero 1.0    |
| `isc`           | ISC License                  |
| `wtfpl`         | Do What The Fuck You Want To |
| `proprietary`   | Proprietary (UNLICENSED)     |

Templates use the [Tera](https://tera.netlify.app/) templating engine with these placeholders:

- `{{ year }}` — copyright year
- `{{ author }}` — copyright holder name
- `{{ company }}` — company name (defaults to author)
- `{{ email }}` — contact email
- `{{ date }}` — current date (ISO 8601)

SPDX-style `<year>`, `<author>`, and `<copyright holders>` placeholders are also supported in both raw and HTML-encoded (`&lt;year&gt;`) forms.

### Custom templates

Add custom template paths in your config:

```toml
[template]
paths = ["/path/to/my/templates"]
```

Project templates in `${PRJ_CONFIG_HOME:-$PRJ_ROOT/.config}/licencify/templates/` take precedence over global templates in `$XDG_CONFIG_HOME/licencify/templates/`, followed by cached/fetched SPDX details and bundled templates. Use `<spdx-id>.tera` for text and `<spdx-id>.html.tera` for HTML or Markdown.

### SPDX API fallback

When no higher-priority custom template exists, Licencify uses cached SPDX detail or fetches it on a cache miss, then falls back to a bundled template. Responses use the global `${XDG_CACHE_HOME:-$HOME/.cache}/licencify/SPDX-Cache/` directory. Cached detail avoids a network request. If no HTML source is available for `html` or `md`, Licencify warns and writes text with a `.txt` extension instead.

### Template cache

```text
Manage global SPDX detail cache

Usage: licencify cache <COMMAND>

Commands:
  clear      Clear all cached SPDX details
  info       Show cache directory location and size
  fetch-all  Pre-fetch and cache all license templates from SPDX
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

## Generating config schema

```text
Generate JSON schema for config file

Usage: licencify schema [OPTIONS]

Options:
  -o, --output <OUTPUT>  Output file path (default: licencify-schema.json) [default: licencify-schema.json]
  -h, --help             Print help
```

Example:

```bash
licencify schema --output licencify-schema.json
```

The generated schema provides validation and auto-completion for your editor.

## Licence

Licencify is released under the [MIT licence](./LICENCE).
