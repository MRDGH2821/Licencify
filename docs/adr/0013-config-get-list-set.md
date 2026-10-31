# Config commands are get, list, and set

`init` is a root command. It scaffolds the project config file, which is what `init` means on other CLIs. `config` only reads and writes fields: `get` prints one key, `list` prints the effective config, and `set` writes one key. `config set` takes `<option>=<value>`. There is no `schema` subcommand. The committed schema file is [ADR 0015](0015-config-schema-is-a-cargo-example.md).

Config keys are hyphenated bare keys, such as `update-readme` and `additional-licences`. Underscore spellings are rejected. [TOML 1.1.0](https://toml.io/en/v1.1.0) allows both underscores and dashes in a bare key, and its examples include `bare_key` and `bare-key`. It does not recommend one of those characters.

## Considered options

- `config init`, `config show`, and a top-level `schema` command.
- `config schema` as a nested command.
- Flags such as `--get` and `--set` instead of subcommands. Git replaced those flag forms with `get`, `list`, and `set`.
- Underscore config keys. TOML allows them and does not prefer them.
