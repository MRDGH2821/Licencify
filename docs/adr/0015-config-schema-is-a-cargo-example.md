# The config schema is a cargo example

A cargo example in this package writes `schema/config.json` by calling the existing schema function. `cargo test` fails when the committed file differs. The same package's `build.rs` cannot call that function, because Cargo runs the script before the library exists. A checked-in generated file is checked by a test, not written from `build.rs`.

The schema URL is <https://raw.githubusercontent.com/MRDGH2821/Licencify/main/schema/config.json>. That address returns the JSON file on the default branch. `init` writes `#:schema` and that URL as the first line when it creates the project config. It still does not overwrite an existing file.

## Considered options

- Emit the file from `build.rs` while building the binary.
- A `schema` command that writes the file when someone runs the binary.
- A separate xtask package. One schema file does not need one.
- A `github.com` blob page as the schema URL. That page is HTML. Editors need the raw JSON file.
