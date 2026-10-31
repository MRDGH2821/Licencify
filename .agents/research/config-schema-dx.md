# Config schema in the editor

How a developer gets completion and diagnostics once a JSON Schema file exists.
How that file is produced is already recorded in
`.agents/research/config-schema-generation.md`. This note does not repeat it.
The settled generation choice is a cargo example that writes `schema/config.json`,
a test that fails if the file drifts, and no schema command on the licencify binary.
Quotes below are from the cited pages. Schema text was fetched on 31 Oct 2026.
Biome's versioned URL is the one that page showed that day.

## mise

The configuration page publishes a URL and names the JSON Schema Store. The
schema section is:

```text
### Editor schema {#mise-toml-schema}

The JSON schema for `mise.toml` is at <https://mise.jdx.dev/schema/mise.json>
and in the [JSON Schema Store](https://www.schemastore.org/), so editors such as
[VS Code](https://code.visualstudio.com/docs/languages/json#_json-schemas-and-settings),
[IntelliJ](https://www.jetbrains.com/help/idea/json.html#ws_json_using_schemas),
and [Neovim](https://github.com/b0o/SchemaStore.nvim) can complete and validate
it. Task files loaded through `task_config.includes` use a separate schema:
<https://mise.jdx.dev/schema/mise-task.json>.
```

<https://mise.jdx.dev/configuration.html>
<https://github.com/jdx/mise/blob/main/docs/configuration.md>

That page does not show a `$schema` key or a `#:schema` comment to paste into
`mise.toml`. The schema document's own `$schema` is the meta-schema, and its
`$id` repeats the published URL:

```text
"$id": "https://mise.jdx.dev/schema/mise.json",
"$schema": "https://json-schema.org/draft/2020-12/schema",
```

<https://mise.jdx.dev/schema/mise.json>
<https://github.com/jdx/mise/blob/main/schema/mise.json>

The same URL is a catalog entry. `fileMatch` lists mise's filenames. The task
schema entry has a `url` and no `fileMatch`:

```text
{
  "name": "mise",
  "description": "mise config, a polyglot dev tool manager",
  "fileMatch": [
    ".mise.toml",
    ".mise.*.toml",
    "mise.toml",
    "mise.*.toml",
    "**/.mise/config.toml",
    "**/.mise/config.*.toml",
    "**/.mise/conf.d/*.toml",
    "**/mise/config.toml",
    "**/mise/config.*.toml",
    "**/mise/conf.d/*.toml"
  ],
  "url": "https://mise.jdx.dev/schema/mise.json"
}
```

```text
{
  "name": "mise task",
  "description": "mise config for `task_config.includes` tasks",
  "url": "https://mise.jdx.dev/schema/mise-task.json"
}
```

<https://github.com/SchemaStore/schemastore/blob/master/src/api/json/catalog.json>

## Biome

Biome's configuration reference tells the user to put a `$schema` string in
`biome.json`. The page says it "Specifies the JSON Schema used to validate the
configuration and provide editor features such as completion and inline
documentation."

If `@biomejs/biome` is installed locally, the page says to "reference the schema
included in the npm package, which always matches the installed Biome version":

```text
{
  "$schema": "./node_modules/@biomejs/biome/configuration_schema.json"
}
```

"If your editor cannot resolve the local file, use the schema published on the
Biome website":

```text
{
  "$schema": "https://biomejs.dev/schemas/2.6.4/schema.json"
}
```

"The version in the `$schema` URL above tracks Biome's latest stable release.
If your project uses a different Biome version, replace it with that version to
get accurate validation and documentation." "Biome will also warn you when your
`$schema` URL version doesn't match the package version."

<https://biomejs.dev/reference/configuration/>

SchemaStore also maps the filenames, to a floating `latest` URL, which is a
different URL from the versioned one in the configuration page:

```text
{
  "name": "Biome Formatter Config",
  "description": "Configuration file for the Biome formatter",
  "fileMatch": ["biome.json", "biome.jsonc"],
  "url": "https://biomejs.dev/schemas/latest/schema.json"
}
```

<https://github.com/SchemaStore/schemastore/blob/master/src/api/json/catalog.json>

## Prettier

The configuration page's schema section is one sentence:

```text
If you'd like a JSON schema to validate your configuration, one is available here: https://www.schemastore.org/prettierrc.json.
```

<https://prettier.io/docs/configuration>
<https://github.com/prettier/prettier/blob/main/docs/configuration.md>

The page does not show a `$schema` line to paste into `.prettierrc`. The catalog
entry that serves that URL matches these names, and the list has no
`.prettierrc.toml`:

```text
{
  "name": "prettierrc.json",
  "description": ".prettierrc configuration file",
  "fileMatch": [
    ".prettierrc",
    ".prettierrc.json",
    ".prettierrc.yml",
    ".prettierrc.yaml"
  ],
  "url": "https://www.schemastore.org/prettierrc.json"
}
```

<https://github.com/SchemaStore/schemastore/blob/master/src/api/json/catalog.json>

## Deno

The configuration reference says:

```text
A JSON schema file is available for editors to provide autocompletion. The file
is versioned and available at:
https://github.com/denoland/deno/blob/main/cli/schemas/config-file.v1.json
```

<https://docs.deno.com/runtime/reference/deno_json/>
<https://github.com/denoland/docs/blob/main/runtime/reference/deno_json.md>

That page does not tell the user to add a `$schema` field to `deno.json`. The
schema file's `$id` is the raw GitHub URL of the same file. Its `$schema` is
the meta-schema. The properties object has no `$schema` field:

```text
"$id": "https://raw.githubusercontent.com/denoland/deno/main/cli/schemas/config-file.v1.json",
"$schema": "https://json-schema.org/draft/2020-12/schema",
```

<https://github.com/denoland/deno/blob/main/cli/schemas/config-file.v1.json>
<https://raw.githubusercontent.com/denoland/deno/main/cli/schemas/config-file.v1.json>

SchemaStore maps the filenames to that raw URL:

```text
{
  "name": "Deno Config (deno.json)",
  "description": "A JSON representation of a Deno configuration file",
  "fileMatch": ["deno.json", "deno.jsonc"],
  "url": "https://raw.githubusercontent.com/denoland/deno/main/cli/schemas/config-file.v1.json"
}
```

<https://github.com/SchemaStore/schemastore/blob/master/src/api/json/catalog.json>

## Taplo

Taplo assigns a schema in this priority order, highest first:

```text
1. set manually in the environment, e.g. as a CLI flag or an IDE setting
1. schema directives at the top of the document
1. as an URL under the `$schema` key in the root of the document
1. configuration file rule
1. default schema set in the configuration file
1. contributed by an extension (Visual Studio Code only)
1. an association based on a schema catalog
```

<https://taplo.tamasfe.dev/configuration/using-schemas.html>
<https://github.com/tamasfe/taplo/blob/master/site/site/configuration/using-schemas.md>

The comment directive is a header comment. "All directive comments must follow
the following pattern: `#:<name> <content>`." "A relative file path or an URL
can be provided." The documented example is:

```text
#:schema ./foo-schema.json
foo = "bar"
```

"Relative paths are relative to the document file, if the file path is not
known, Taplo will be unable to find the schema." "Multiple schema directives
in the same document are not supported and the behaviour is undefined."

<https://taplo.tamasfe.dev/configuration/directives.html>
<https://github.com/tamasfe/taplo/blob/master/site/site/configuration/directives.md>

A separate Taplo config (`.taplo.toml` or `taplo.toml`) can set the schema
without a line in the target file. The documented `schema` table is `path` plus
optional `enabled`. Examples from that page:

```text
[schema]
path = "./path/to/schema.json"
enabled = true
```

```text
[schema]
path = "https://example.com/my_schema.json"
```

Rules can override that per include pattern. The CLI flag form is:

```text
taplo check --schema https://example.com/foo-schema.json foo.toml
```

Catalog matching is off unless the user passes `--schema-catalog` or
`--default-schema-catalogs`.

<https://taplo.tamasfe.dev/configuration/file.html>
<https://github.com/tamasfe/taplo/blob/master/site/site/configuration/file.md>
<https://taplo.tamasfe.dev/cli/usage/validation.html>
<https://github.com/tamasfe/taplo/blob/master/site/site/cli/usage/validation.md>

`taplo config schema` is not part of those association steps. The command's
implementation writes `schema_for!(Config)` to stdout. None of the directive,
using-schemas, configuration-file, developing-schemas, validation, or Even
Better TOML readme pages tell the user to run it.

<https://github.com/tamasfe/taplo/blob/master/crates/taplo-cli/src/commands/config.rs>

The maintainer's note on the directive versus the key: "Both behave the same,
`$schema` is actually part of the data though so it will be visible to most
TOML libraries and tools and will be preserved by tools that strip comments."

<https://github.com/tamasfe/taplo/issues/255>

Even Better TOML is the VS Code extension in this repo. Its readme says:
"Schemas can be associated with document URIs with the
`evenBetterToml.schema.associations` configuration." "You can provide your own
schemas or use existing schemas from the JSON Schema Store." The setting value
"must be an absolute URI to the JSON schema". The key is a regular expression
matched against absolute document URIs. Schema catalogs default to one URL:

```text
"evenBetterToml.schema.catalogs" default:
[
  "https://www.schemastore.org/api/json/catalog.json"
]
```

<https://github.com/tamasfe/taplo/blob/master/editors/vscode/README.md>
<https://github.com/tamasfe/taplo/blob/master/editors/vscode/package.json>
<https://taplo.tamasfe.dev/>

Extensions can also contribute `tomlValidation` entries with `fileMatch` or
`regexMatch` and a `url`. "Submitting schemas directly to Taplo is not possible
anymore, and all JSON schemas should be submitted to the JSON Schema Store."

<https://taplo.tamasfe.dev/configuration/developing-schemas.html>
<https://github.com/tamasfe/taplo/blob/master/site/site/configuration/developing-schemas.md>

## SchemaStore

The site describes itself as "the central catalog for JSON, YAML, and TOML
schemas." "Editors and tools use it for validation, completion, and in-context
documentation." "These editors load SchemaStore automatically for JSON, YAML,
and TOML. A schema merged here shows up in the next sync." The VS Code row on
that list links the TOML editor to Even Better TOML.

<https://www.schemastore.org/>

The public API example maps filenames to an absolute schema URL:

```text
{
  "name": "bower.json",
  "description": "Bower package description file",
  "fileMatch": [ "bower.json", ".bower.json" ],
  "url": "https://www.schemastore.org/bower.json"
}
```

"`url` property is an absolute URI pointing to the schema. It can be hosted
anywhere on the web." "`fileMatch` property is for specifying what known file
names corresponds with the schema. This property is optional since not all JSON
formats enforce a specific file name."

<https://www.schemastore.org/>

The catalog file in the repository is `src/api/json/catalog.json`. One TOML
mapping in that file is the mise entry quoted above. A self-hosted schema is
registered the same way, with `url` pointing outside SchemaStore. The
contributing guide's example is ORY Hydra, `fileMatch` including `hydra.toml`,
and this `url`:

```text
https://raw.githubusercontent.com/ory/hydra/master/.schema/version.schema.json
```

<https://github.com/SchemaStore/schemastore/blob/master/src/api/json/catalog.json>
<https://github.com/SchemaStore/schemastore/blob/master/CONTRIBUTING.md>

A catalog entry is how filename auto-detection works for tools that read the
catalog. It is not the only way to attach a schema. Taplo's higher-priority
methods are the directive, the `$schema` key, a Taplo config rule, and an IDE
setting. The contributing guide says: "The `catalog.json` file is generally
used by IDEs and language servers to determine which schemas apply to what
files." It then says "VSCode ignores this file" and links
<https://github.com/microsoft/vscode/issues/26289>, and "Taplo TOML language
server uses this file." The Taplo link target is the extension's default
catalog setting.

<https://github.com/SchemaStore/schemastore/blob/master/CONTRIBUTING.md>
<https://github.com/tamasfe/taplo/blob/2e01e8cca235aae3d3f6d4415c06fd52e1523934/editors/vscode/package.json#L240>

Generic names are rejected. The guide's example is Hugo's old `config.toml`
pattern, with the comment "Avoid generic patterns." "This would not be accepted
because the file detection would have too many false positives." The listed
fixes are a more specific filename, an empty or omitted `fileMatch` ("which
still allows the user to manually select it"), or a directory prefix such as
`"**/micro/runtime/syntax/*.yaml"`.

<https://github.com/SchemaStore/schemastore/blob/master/CONTRIBUTING.md>

## VS Code JSON docs

The JSON language page documents association for JSON files. "The association
of a JSON file to a schema can be done either in the JSON file itself using the
`$schema` attribute, or in the User or Workspace settings (`json.schemas`)."
The in-file example is:

```text
{
  "$schema": "https://json.schemastore.org/coffeelint",
  "line_endings": "unix"
}
```

"Note that this syntax is VS Code-specific and not part of the JSON Schema
specification. Adding the `$schema` key changes the JSON itself, which systems
consuming the JSON might not expect, for example, schema validation might fail.
If this is the case, you can use one of the other mapping methods."

A workspace file is a relative `url` inside `json.schemas`, for example
`"./myschema.json"` for a schema in the workspace root. The page fetched on
31 Oct 2026 does not contain the word TOML. The TOML association comment and
the `evenBetterToml.schema.associations` setting are in the Taplo sources above.
mise's editor section links this JSON page for VS Code.

<https://code.visualstudio.com/docs/languages/json#_json-schemas-and-settings>
<https://code.visualstudio.com/docs/languages/json#_mapping-in-the-json>
<https://code.visualstudio.com/docs/languages/json#_mapping-to-a-schema-in-the-workspace>

## Licencify paths

The shared project config is
`${PRJ_CONFIG_HOME:-$PRJ_ROOT/.config}/licencify/config.toml`. The adjacent
local file is `config.local.toml`. The global file is
`$XDG_CONFIG_HOME/licencify/config.toml` or the platform config directory.
Root-level `.licencify.toml` and `licencify.toml` are not loaded.

<https://github.com/MRDGH2821/Licencify/blob/0bba017180adbcf85d52465d92294d970edbe494/README.md>

`project_config_dir` uses `PRJ_CONFIG_HOME` when set, otherwise
`$PRJ_ROOT/.config`, then joins `licencify`. `project_path` joins `config.toml`.
`global_path` joins `licencify/config.toml` under the user config directory.

<https://github.com/MRDGH2821/Licencify/blob/0bba017180adbcf85d52465d92294d970edbe494/src/config.rs>

Unknown keys are warned. Allowed top-level keys are `default`, `template`,
`subdirs`, and `scan` in a project file, and `default` and `template` in the
global file. A root key named `$schema` is outside that list.

<https://github.com/MRDGH2821/Licencify/blob/0bba017180adbcf85d52465d92294d970edbe494/src/config.rs>

ADR 0015 names the committed artifact `schema/config.json` and rejects "a
`schema` command that writes the file when someone runs the binary." This
commit's tree has no `schema/` directory. The same checkout still declares a
`Schema` subcommand in `src/cli.rs` ("Generate JSON schema for config file").

<https://github.com/MRDGH2821/Licencify/blob/0bba017180adbcf85d52465d92294d970edbe494/docs/adr/0015-config-schema-is-a-cargo-example.md>
<https://github.com/MRDGH2821/Licencify/blob/0bba017180adbcf85d52465d92294d970edbe494/src/cli.rs>

This repo already points Tombi at published schema URLs from
`.config/tombi.toml`, including mise:

```text
[[schemas]]
path = "https://mise.jdx.dev/schema/mise.json"
include = [
  "**/.config/mise/config.toml",
  "**/.mise.toml",
  "**/mise.toml",
  ".config/mise/config.toml",
  ".mise.toml",
  "mise.toml"
]
```

<https://github.com/MRDGH2821/Licencify/blob/0bba017180adbcf85d52465d92294d970edbe494/.config/tombi.toml>

## Answers

1. What the docs tell the user to put in the config file:

- mise: no line. The page gives `https://mise.jdx.dev/schema/mise.json` and the
  Schema Store.
- Biome: `"$schema"` in the JSON file, either
  `./node_modules/@biomejs/biome/configuration_schema.json` or
  `https://biomejs.dev/schemas/<version>/schema.json`.
- Prettier: no line. The page gives `https://www.schemastore.org/prettierrc.json`.
- Deno: no line. The page links the committed schema on GitHub. The schema
  `$id` is the raw URL of that file.
- Taplo: a header comment `#:schema <relative path or URL>`, or a root key
  `$schema` whose value is a URL. Either is optional. A `.taplo.toml` rule, an
  IDE setting, an extension contribution, or a catalog `fileMatch` can attach
  the schema with nothing in the target file.

2. A committed schema file is the artifact Biome's relative `$schema` path and
   Deno's docs link name. The editor still needs a pointer. mise, Biome's
   fallback, Prettier, and Deno's `$id` publish a URL. SchemaStore `url` must
   be an absolute URI, so a path inside the repo is not a catalog URL until it
   is hosted (the Hydra example uses a raw GitHub URL). Even Better TOML's
   `evenBetterToml.schema.associations` value must be an absolute URI. Taplo's
   `#:schema` directive is the method that accepts a path relative to the TOML
   file, so a committed file can be named from the document without a public
   URL, when Taplo knows the document path.

3. For
   `${PRJ_CONFIG_HOME:-$PRJ_ROOT/.config}/licencify/config.toml`, the match to
   these sources is Taplo's `#:schema` comment or a catalog `fileMatch` whose
   pattern includes the `licencify` directory, plus an absolute schema URL if
   the catalog or the Even Better TOML setting is the pointer. A relative
   `#:schema` path is resolved from the config file's directory. That reaches
   a repo-root `schema/config.json` only when the file is at the default
   `$PRJ_ROOT/.config/licencify/config.toml`. `PRJ_CONFIG_HOME`, the global
   file, and `config.local.toml` are other files and other directories. Each
   document needs its own directive, or a `fileMatch` / editor rule that
   covers that path. A root `$schema` key is the Taplo method the VS Code JSON
   page also warns about: it changes the document. This parser would warn on
   it as an unknown key. A bare `fileMatch` of `config.toml` is the pattern
   SchemaStore's contributing guide rejects.

   Reopens the settled "no schema command" decision: a `licencify schema` or
   `licencify config schema` command so an editor can obtain the schema by
   running the binary. `taplo config schema` only prints Taplo's own config
   schema. The association docs do not require it. Copying it into Licencify
   is the command ADR 0015 rejects. The `Schema` variant still declared in
   `src/cli.rs` is that reopening item. A `#:schema` line, a published URL, a
   catalog entry, and `evenBetterToml.schema.associations` do not require it.

4. Recommendation, as a reading of these sources, not a Licencify decision.
   Point editors at the committed schema with Taplo's `#:schema` comment, using
   a URL once one is published, the same way mise, Biome's fallback, Prettier,
   and Deno expose a URL and the way this repo already points Tombi at mise's
   URL. Use a directory-qualified catalog `fileMatch` only if auto-detection
   by filename is wanted. Leave the schema command out.
