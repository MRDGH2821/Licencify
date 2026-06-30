# Licencify

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

```
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
          Output format: txt (default) or html

          Possible values:
          - txt:  Plain text (licenseText)
          - html: HTML (licenseTextHtml)

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
          Output format: txt (default) or html

          Possible values:
          - txt:  Plain text (licenseText)
          - html: HTML (licenseTextHtml)

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

- **Global config**: `~/.config/licencify/config.toml` (Linux) or equivalent per `dirs::config_dir()`
- **Project config**: `.licencify.toml` or `licencify.toml` in the project root (walked up from CWD)
- **Subdirectory overrides**: defined inside the config under `[subdirs]`

Global defaults are overridden by project config, which is overridden by CLI flags.

### Subdirectory overrides

You can set different licence values for specific subdirectories:

```toml
[default]
author = "Jane Doe"
license = "MIT"

[[subdirs]]
path = "vendor"
author = "Third Party"
license = "BSD-3-Clause"

[[subdirs]]
path = "docs"
license = "CC0-1.0"
```

## Licence templates

Licencify ships with 14 built-in template pairs (plain text + HTML):

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

Custom templates are checked before built-in ones. Name your files `<spdx-id>.tera` (plain text) and `<spdx-id>.html.tera` (HTML).

### SPDX API fallback

For licences without a built-in template, licencify fetches the full licence text from `https://spdx.org/licenses/<id>.json`. Responses are cached locally in the XDG cache directory.

### Template cache

```text
Manage local template cache

Usage: licencify cache <COMMAND>

Commands:
  clear      Clear all cached templates
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
