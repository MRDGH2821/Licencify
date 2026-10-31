# Project licences are applied with one command

`licencify apply` creates or replaces the project's licence files. No id uses the effective config and does not change it. One id sets `licence` and leaves `additional-licences` unchanged. Two or more ids set `licence` to the first and replace `additional-licences` with the rest.

From the project root, config is written to the shared section. From a subdirectory, it is written to an exact-path `[[subdirs]]` entry. `--root` writes the shared section from either place. There is no destination prompt.

## Considered options

- Keep `add` and `update` as two commands.
- `--shared` and `--subdir`, or a prompt that asks which section to write.
- Append the extra ids onto `additional-licences`.
