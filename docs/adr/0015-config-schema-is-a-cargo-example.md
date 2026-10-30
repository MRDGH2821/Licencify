# The config schema is a cargo example

A cargo example in this package writes `schema/config.json` by calling the existing schema function. `cargo test` fails when the committed file differs. The same package's `build.rs` cannot call that function, because Cargo runs the script before the library exists. A checked-in generated file is checked by a test, not written from `build.rs`.

## Considered options

- Emit the file from `build.rs` while building the binary.
- A `schema` command that writes the file when someone runs the binary.
- A separate xtask package. One schema file does not need one.
