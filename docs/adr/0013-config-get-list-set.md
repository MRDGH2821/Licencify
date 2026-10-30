# Config commands are get, list, and set

`init` is a root command. It scaffolds the project config file, which is what `init` means on other CLIs. `config` only reads and writes fields: `get` prints one key, `list` prints the effective config, and `set` writes one key. There is no `schema` subcommand. The JSON schema is a generated file committed in the repo. The step that writes that file is not settled.

## Considered options

- `config init`, `config show`, and a top-level `schema` command.
- `config schema` as a nested command.
- Flags such as `--get` and `--set` instead of subcommands. Git replaced those flag forms with `get`, `list`, and `set`.
