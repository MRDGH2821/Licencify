# Config schema generation

Primary sources only: official docs, each project's repository files, the Cargo book, and the schemars docs. Commands and paths below are copied from those sources.

## Dominant pattern

- A committed schema file is written by a separate program, then checked or regenerated. mise's task `mise run render:schema` runs `cargo run --quiet --ignore-rust-version --package mise-config --example render-schema`, and that example writes `schema/mise.json`, `schema/mise-task.json`, and `schema/miserc.json`. The unit test
  `schema_files_are_up_to_date` fails when a committed file differs and tells the reader to run `mise run render:schema`. <https://github.com/jdx/mise/blob/main/xtasks/render/schema.sh> <https://github.com/jdx/mise/blob/main/crates/mise-config/examples/render-schema.rs> <https://github.com/jdx/mise/blob/main/crates/mise-config/src/schema.rs>
- Biome does the same with another package, not `biome_configuration`'s `build.rs`. `cargo codegen-schema` is the alias `run -p xtask_codegen --features schema -- schema`. `generate_configuration_schema` calls `schema_for!(Configuration)` and writes `packages/@biomejs/biome/configuration_schema.json`. `.github/workflows/autofix.yml` runs
  `cargo codegen-schema`. <https://github.com/biomejs/biome/blob/main/.cargo/config.toml> <https://github.com/biomejs/biome/blob/main/xtask/codegen/src/generate_schema.rs> <https://github.com/biomejs/biome/blob/main/.github/workflows/autofix.yml>
- Several tools print the schema to stdout instead of writing a repository file. taplo's `ConfigCommand::Schema` writes `schema_for!(Config)` to stdout (`taplo config schema`; binary name `taplo`). rust-analyzer prints `Config::json_schema()` for `rust-analyzer lsp-server --print-config-schema`. Terraform's documented command is
  `terraform providers schema -json`. <https://github.com/tamasfe/taplo/blob/master/crates/taplo-cli/src/commands/config.rs> <https://github.com/tamasfe/taplo/blob/master/crates/taplo-cli/src/args.rs> <https://github.com/tamasfe/taplo/blob/master/crates/taplo-cli/Cargo.toml>
  <https://github.com/rust-lang/rust-analyzer/blob/master/crates/rust-analyzer/src/bin/main.rs> <https://developer.hashicorp.com/terraform/cli/commands/providers/schema>
- Editors are also given a URL, or a file shipped beside the tool. mise documents `https://mise.jdx.dev/schema/mise.json`. Biome documents `./node_modules/@biomejs/biome/configuration_schema.json` and `https://biomejs.dev/schemas/<version>/schema.json`. Prettier's docs give only `https://www.schemastore.org/prettierrc.json`. Deno's docs
  link to the committed file `cli/schemas/config-file.v1.json`. <https://mise.jdx.dev/configuration.html> <https://biomejs.dev/reference/configuration/> <https://prettier.io/docs/configuration> <https://docs.deno.com/runtime/reference/deno_json/>
- None of these writers is the `build.rs` of the package whose types the schema describes. mise's contributing guide says `crates/mise-settings/build.rs` generates the Rust `Settings` type from `settings.toml`; the JSON files come from `mise run render:schema`. <https://github.com/jdx/mise/blob/main/docs/contributing.md>

## Tools

| tool          | mechanism                                                                                              | exact command or file                                                                                                                                                           | source URL                                                                                     |
| ------------- | ------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| mise          | separate example binary writes three committed JSON files; `schema_files_are_up_to_date` compares them | `mise run render:schema` (full `cargo run` is in the note above)                                                                                                                | <https://github.com/jdx/mise/blob/main/xtasks/render/schema.sh>                                |
| mise          | published URL                                                                                          | `https://mise.jdx.dev/schema/mise.json` (task files: `https://mise.jdx.dev/schema/mise-task.json`)                                                                              | <https://mise.jdx.dev/configuration.html>                                                      |
| biome         | separate xtask writes the file committed in the npm package; CI runs the alias                         | `cargo codegen-schema` = `cargo run -p xtask_codegen --features schema -- schema`; writes `packages/@biomejs/biome/configuration_schema.json` with `schema_for!(Configuration)` | <https://github.com/biomejs/biome/blob/main/xtask/codegen/src/generate_schema.rs>              |
| biome         | npm package file and versioned URL                                                                     | `configuration_schema.json` in the npm package, plus a versioned docs URL                                                                                                       | <https://biomejs.dev/reference/configuration/>                                                 |
| taplo         | subcommand prints schemars output                                                                      | `taplo config schema` (`ConfigCommand::Schema` writes `serde_json::to_string_pretty(&schema_for!(Config))` to stdout)                                                           | <https://github.com/tamasfe/taplo/blob/master/crates/taplo-cli/src/commands/config.rs>         |
| rust-analyzer | flag prints the LSP config schema                                                                      | `rust-analyzer lsp-server --print-config-schema` prints `Config::json_schema()`                                                                                                 | <https://github.com/rust-lang/rust-analyzer/blob/master/crates/rust-analyzer/src/cli/flags.rs> |
| prettier      | script prints JSON; docs publish a URL; release step compares SchemaStore                              | `scripts/generate-schema.js` prints the schema; a release step compares SchemaStore                                                                                             | <https://prettier.io/docs/configuration>                                                       |
| deno          | committed JSON file; docs link to it; no generator script found                                        | `cli/schemas/config-file.v1.json` (`$id` is the raw GitHub URL of that file). A code search for `config-file.v1` returned only this file                                        | <https://docs.deno.com/runtime/reference/deno_json/>                                           |
| terraform     | subcommand prints provider schemas                                                                     | `terraform providers schema -json`                                                                                                                                              | <https://developer.hashicorp.com/terraform/cli/commands/providers/schema>                      |
| kubectl       | runtime API, not a file in the repo                                                                    | `kubectl explain` says field information "is retrieved from the server in OpenAPI format". The API server serves OpenAPI at `/openapi/v2`                                       | <https://kubernetes.io/docs/reference/kubectl/generated/kubectl_explain/>                      |

## Same package `build.rs`

A `build.rs` in a package runs before that package's library is compiled, and it is a separate crate. The Cargo book says: "Just before a package is built, Cargo will compile a build script into an executable... Once the build script successfully finishes executing, the rest of the package will be compiled." Under build dependencies: "The
build script does not have access to the dependencies listed in the `dependencies` or `dev-dependencies` section (they're not built yet!)." <https://doc.rust-lang.org/cargo/reference/build-scripts.html>

The same book, on specifying dependencies: "A package itself and its build script are built separately, so their dependencies need not coincide." <https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html>

The resolver "does not allow cycles in the graph", with an exception for dev-dependency cycles. <https://doc.rust-lang.org/cargo/reference/resolver.html>

For a generated file that is checked in, the Cargo book says the file should not be generated from `build.rs`. It says to use a test that fails when the checked-in file does not match, and to run that test in CI. <https://doc.rust-lang.org/cargo/reference/build-script-examples.html>

schemars documents `schema_for!` as the way to build a schema for a type that implements `JsonSchema`. The examples call it from `main` and print the JSON. The generating page does not mention `build.rs`. A code search of `GREsau/schemars` for `build.rs` or "build script" returned no files. <https://docs.rs/schemars/latest/schemars/>
<https://graham.cool/schemars/generating/> <https://github.com/GREsau/schemars>

So `schema_for!` runs in a crate that can already name the type. The `build.rs` of that same package cannot name the package's library types: the script is compiled and executed first, and it only links `[build-dependencies]`.

## Checked, no config-schema writer found

- cargo-deny: a recursive listing of the `main` tree had no path containing `schema`. <https://github.com/EmbarkStudios/cargo-deny>
- ESLint's configuration-file docs do not document a JSON schema for the config file. The custom-rules docs say a rule's `meta.schema` is "a JSON Schema format description of a rule's options". <https://eslint.org/docs/latest/extend/custom-rules> <https://eslint.org/docs/latest/use/configure/configuration-files>
