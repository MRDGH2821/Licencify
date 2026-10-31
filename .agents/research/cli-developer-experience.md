# CLI developer experience

This note compares Licencify's decided command surface with the command
grammars, help, and examples in the official manuals of git, gh, cargo, npm,
kubectl, and Hugo, plus the REUSE tool manual and the Command Line Interface
Guidelines. [clig.dev](https://clig.dev/) is that guideline document, written
and published by its authors. It is not a secondary roundup.

Licencify citations point at the product spec and ADRs on branch
`MRDGH2821/docs/cli-commands`. Quotes use straight ASCII quotation marks.
Each command form is copied from the cited page.

The Rust CLI in this repository is out of scope. It has not been updated to
this surface.

## Settled Licencify surface

Root commands are `init`, `list`, `search`, `detect`, `scan`, `apply`,
`cache`, and `config`. There is no `schema` subcommand. A cargo example
writes `schema/config.json`, and `cargo test` fails if that file drifts.

- `init` creates only the root shared config and does not overwrite it.
- `config get <key>`, `config list`, and `config set <key> <value>`. No
  `config show`. No `config init`.
- `cache` with no subcommand prints the directory, file count, and size.
  `cache update` refreshes `licenses.json` only. `cache fetch` with no id
  downloads every uncached detail JSON. `cache fetch <id>` downloads that
  one id and does not refetch a cached copy. `cache clear` removes only the
  SPDX cache directory. No `cache info`. No `--update` or `--fetch` flags.
- `apply [licence]...` replaces `add` and `update`. No id writes licence
  files from the effective config and does not change config. One id sets
  `licence` and leaves `additional-licences`, except when
  `--permit-promotion` removes that id. Two or more ids set the first as
  `licence` and replace `additional-licences` with the rest. A repeated id
  is an error. `proprietary` cannot mix with another id. From the project
  root, write the shared section. From a subdirectory, write an exact-path
  `[[subdirs]]` entry. `--root` writes the shared section from either place.
  There is no destination prompt. If one primary file exists, confirm before
  replacing it. `--yes` skips that prompt. Several primary candidates fail
  with no changes. Flags that stay include `author`, `company`, `email`,
  `year`, `format`, `--yes`, `--permit-promotion`, `--no-file`,
  `--update-readme` / `--no-update-readme`, and `--update-manifest` /
  `--no-update-manifest`.
- No `remove` command. Deleting a licence file is out of scope.

Sources:
[product spec](https://github.com/MRDGH2821/Licencify/blob/main/docs/superpowers/specs/2026-09-30-licencify-product-spec.md),
[ADR 0013](https://github.com/MRDGH2821/Licencify/blob/main/docs/adr/0013-config-get-list-set.md),
[ADR 0014](https://github.com/MRDGH2821/Licencify/blob/main/docs/adr/0014-cache-update-and-fetch.md),
[ADR 0015](https://github.com/MRDGH2821/Licencify/blob/main/docs/adr/0015-config-schema-is-a-cargo-example.md),
[ADR 0016](https://github.com/MRDGH2821/Licencify/blob/main/docs/adr/0016-project-licences-are-applied.md).

## Command forms

| tool    | form copied from the manual                                                            | source                                                                                      |
| ------- | -------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| git     | `git init [-q \| --quiet] ... [<directory>]`                                           | <https://git-scm.com/docs/git-init>                                                         |
| git     | `git config list`, `git config get ... <name>`, `git config set ... <name> <value>`    | <https://git-scm.com/docs/git-config>                                                       |
| git     | `git add [--verbose \| -v] [--dry-run \| -n] [--force \| -f] ... [--] [<pathspec>...]` | <https://git-scm.com/docs/git-add>                                                          |
| git     | `git apply [--stat] [--check] ... [<patch>...]`                                        | <https://git-scm.com/docs/git-apply>                                                        |
| git     | `git [-v \| --version] [-h \| --help] [-C <path>] ... <command> [<args>]`              | <https://git-scm.com/docs/git>                                                              |
| git     | `git-fetch`: "Download objects and refs from another repository"                       | <https://git-scm.com/docs/git>                                                              |
| git     | `git-rm`: "Remove files from the working tree and from the index"                      | <https://git-scm.com/docs/git>                                                              |
| git     | `git-help`: "Display help information about Git"                                       | <https://git-scm.com/docs/git>                                                              |
| gh      | `gh help [command]`                                                                    | <https://cli.github.com/manual/gh_help>                                                     |
| gh      | `gh config set <key> <value> [flags]`                                                  | <https://cli.github.com/manual/gh_config_set>                                               |
| gh      | `gh config` commands: `clear-cache`, `get`, `list`, `set`                              | <https://cli.github.com/manual/gh_config>                                                   |
| gh      | `gh repo delete [<repository>] [flags]` with `--yes`                                   | <https://cli.github.com/manual/gh_repo_delete>                                              |
| gh      | `gh search` with `code`, `commits`, `issues`, `prs`, `repos`                           | <https://cli.github.com/manual/gh_search>                                                   |
| cargo   | `cargo init [options] [path]`                                                          | <https://doc.rust-lang.org/cargo/commands/cargo-init.html>                                  |
| cargo   | `cargo new [options] path`                                                             | <https://doc.rust-lang.org/cargo/commands/cargo-new.html>                                   |
| cargo   | `cargo add [options] crate...`                                                         | <https://doc.rust-lang.org/cargo/commands/cargo-add.html>                                   |
| cargo   | `cargo fetch [options]`                                                                | <https://doc.rust-lang.org/cargo/commands/cargo-fetch.html>                                 |
| cargo   | `cargo search [options] [query...]`                                                    | <https://doc.rust-lang.org/cargo/commands/cargo-search.html>                                |
| npm     | `npm init <package-spec>` (aliases `create`, `innit`)                                  | <https://docs.npmjs.com/cli/v11/commands/npm-init/>                                         |
| npm     | `npm config set <key>=<value> [<key>=<value> ...]`                                     | <https://docs.npmjs.com/cli/v11/commands/npm-config/>                                       |
| npm     | `npm config get [<key> [<key> ...]]`                                                   | <https://docs.npmjs.com/cli/v11/commands/npm-config/>                                       |
| npm     | `npm config list [--json]`                                                             | <https://docs.npmjs.com/cli/v11/commands/npm-config/>                                       |
| npm     | `npm search <search term> [<search term> ...]` (aliases `find`, `s`, `se`)             | <https://docs.npmjs.com/cli/v11/commands/npm-search/>                                       |
| npm     | `npm prefix`                                                                           | <https://docs.npmjs.com/cli/v11/commands/npm-prefix/>                                       |
| npm     | `npm cache add`, `clean`, `ls`, `verify`                                               | <https://docs.npmjs.com/cli/v11/commands/npm-cache/>                                        |
| kubectl | `kubectl apply (-f FILENAME \| -k DIRECTORY)`                                          | <https://kubernetes.io/docs/reference/kubectl/generated/kubectl_apply/>                     |
| kubectl | `kubectl create -f FILENAME`                                                           | <https://kubernetes.io/docs/reference/kubectl/generated/kubectl_create/>                    |
| kubectl | `kubectl config set PROPERTY_NAME PROPERTY_VALUE`                                      | <https://kubernetes.io/docs/reference/kubectl/generated/kubectl_config/kubectl_config_set/> |
| hugo    | `hugo [flags]`                                                                         | <https://gohugo.io/commands/hugo/>                                                          |
| hugo    | `hugo config [command] [flags]`                                                        | <https://gohugo.io/commands/hugo_config/>                                                   |
| hugo    | `hugo new`                                                                             | <https://gohugo.io/commands/hugo_new/>                                                      |
| reuse   | `reuse [options] <command>`                                                            | <https://reuse.readthedocs.io/en/stable/man/reuse.html>                                     |
| reuse   | `reuse annotate [options] [file ...]`                                                  | <https://reuse.readthedocs.io/en/stable/man/reuse-annotate.html>                            |
| reuse   | `reuse download [options] [license ...]`                                               | <https://reuse.readthedocs.io/en/stable/man/reuse-download.html>                            |
| reuse   | `reuse lint [options]`                                                                 | <https://reuse.readthedocs.io/en/stable/man/reuse-lint.html>                                |

Hugo's command index on the `hugo` page also lists `hugo list`,
`hugo new project`, and `hugo mod init`. It does not list `detect`, `scan`,
`search`, `apply`, or `schema`.
<https://gohugo.io/commands/hugo/>

REUSE's command list is `annotate`, `convert-dep5`, `download`, `lint`,
`lint-file`, `spdx`, and `supported-licenses`.
<https://reuse.readthedocs.io/en/stable/man/reuse.html>

## 1. Which verbs match, and which are unusual?

### They match the dominant verb

`init` matches the scaffold verb on git, cargo, and npm. The git-init name
line is "Create an empty Git repository or reinitialize an existing one."
The description says "Running `git init` in an existing repository is safe.
It will not overwrite things that are already there."
<https://git-scm.com/docs/git-init>
cargo-init's description says "This command will create a new Cargo
manifest in the current directory."
<https://doc.rust-lang.org/cargo/commands/cargo-init.html>
`npm init` with no initializer will "write a package.json for you" and "is
strictly additive, so it will keep any fields and values that were already
set."
<https://docs.npmjs.com/cli/v11/commands/npm-init/>
Hugo's page uses `hugo new` ("Create new content") and lists
`hugo new project`, not a root `init`, for that job.
<https://gohugo.io/commands/hugo_new/>
<https://gohugo.io/commands/hugo/>
Licencify's non-overwriting `init` matches git and npm. Hugo is the outlier.

`list` for many items and `search` for a query match these manuals.
`git config list` is "List all variables set in config file, along with
their values."
<https://git-scm.com/docs/git-config>
`gh config list` is one of the `gh config` commands.
<https://cli.github.com/manual/gh_config>
`npm config list` is "Show all the config settings."
<https://docs.npmjs.com/cli/v11/commands/npm-config/>
`cargo search` "performs a textual search for crates."
<https://doc.rust-lang.org/cargo/commands/cargo-search.html>
`npm search` is "Search the registry for packages matching the search
terms."
<https://docs.npmjs.com/cli/v11/commands/npm-search/>
`gh search` is "Search across all of GitHub."
<https://cli.github.com/manual/gh_search>
Licencify `list` (identifiers, no query) and `search <query>` follow that
split. Hugo's `hugo list` is an inventory of content, not a query.
<https://gohugo.io/commands/hugo/>

`config get`, `config list`, and `config set` match git, gh, and npm
exactly, including `get <key>` and `set <key> <value>`:

- `git config get ... <name>` and `git config set ... <name> <value>`
  <https://git-scm.com/docs/git-config>
- `gh config set <key> <value>`
  <https://cli.github.com/manual/gh_config_set>
- `npm config get` and `npm config set <key>=<value>` (the page also accepts
  `npm config set key value`)
  <https://docs.npmjs.com/cli/v11/commands/npm-config/>

`kubectl config set PROPERTY_NAME PROPERTY_VALUE` matches `set`. The pages
fetched here do not document `kubectl config get` or `kubectl config list`.
<https://kubernetes.io/docs/reference/kubectl/generated/kubectl_config/kubectl_config_set/>
`hugo config` is "Display project configuration, both default and custom
settings" with no `get`, `list`, or `set` child on that page.
<https://gohugo.io/commands/hugo_config/>
Omitting `config show` and `config init` matches git, gh, and npm. Their
pages use `get` and `list`, and they keep `init` as its own command.

`apply` matches kubectl's create-or-update verb. The apply page says "This
resource will be created if it doesn't exist yet." The conventions page
says "You can use `kubectl apply` to create or update resources."
<https://kubernetes.io/docs/reference/kubectl/generated/kubectl_apply/>
<https://kubernetes.io/docs/reference/kubectl/conventions/>
It does not match `git apply`, which "Apply a patch to files and/or to the
index."
<https://git-scm.com/docs/git-apply>

`cache fetch` uses the same verb as `cargo fetch` ("Fetch dependencies of a
package from the network") and as git's `git-fetch` ("Download objects and
refs from another repository").
<https://doc.rust-lang.org/cargo/commands/cargo-fetch.html>
<https://git-scm.com/docs/git>
`cache clear` sits next to `gh config clear-cache`.
<https://cli.github.com/manual/gh_config>
npm's removal verb is `clean`, not `clear`: `npm cache clean`.
<https://docs.npmjs.com/cli/v11/commands/npm-cache/>

No fetched page in this set documents a `schema` subcommand. Leaving schema
generation to a cargo example matches that absence.
<https://gohugo.io/commands/hugo/>
<https://reuse.readthedocs.io/en/stable/man/reuse.html>

### They are unusual in this set

`detect` and `scan` do not appear in the Hugo command index or the REUSE
command list. REUSE's project check is `reuse lint`. Its single-file check
is `reuse lint-file`.
<https://gohugo.io/commands/hugo/>
<https://reuse.readthedocs.io/en/stable/man/reuse.html>
<https://reuse.readthedocs.io/en/stable/man/reuse-lint.html>

`apply` is the unusual word next to `git apply` (a patch), `cargo add`
("can add or modify dependencies"), `npm init`, `hugo new`, and
`reuse annotate` / `reuse download`. It is the usual word only in the
kubectl manual among these tools.
<https://git-scm.com/docs/git-apply>
<https://doc.rust-lang.org/cargo/commands/cargo-add.html>
<https://reuse.readthedocs.io/en/stable/man/reuse-annotate.html>
<https://reuse.readthedocs.io/en/stable/man/reuse-download.html>

A bare `cache` that prints directory, count, and size is not how `npm cache`
is specified. That synopsis requires `add`, `clean`, `ls`, `verify`, or an
`npx` child. `npm cache ls` lists entries. `npm cache npx info` is the info
verb, and it takes keys.
<https://docs.npmjs.com/cli/v11/commands/npm-cache/>
`gh config` with no child is described as "Display or change configuration
settings", then lists subcommands, rather than printing a cache summary.
<https://cli.github.com/manual/gh_config>
The closer pattern for a bare command that prints state is `hugo config`.
<https://gohugo.io/commands/hugo_config/>

`cache update` as "refresh the index only" is not an npm cache subcommand.
Cargo's fetched `fetch` page downloads dependencies named by the lockfile.
It does not describe an index-only update, and it does not say that a cached
copy is left untouched.
<https://docs.npmjs.com/cli/v11/commands/npm-cache/>
<https://doc.rust-lang.org/cargo/commands/cargo-fetch.html>

`remove` is a normal git verb (`git-rm`). The REUSE pages cited here do not
document a remove command. Licencify's lack of `remove` matches those REUSE
pages and is unusual next to git.
<https://git-scm.com/docs/git>
<https://reuse.readthedocs.io/en/stable/man/reuse.html>

## 2. How do they treat a command that both creates and replaces?

They usually keep one verb and spell the replace rule in the description.
They do not split "create" and "replace" into two commands for the same
object.

- kubectl: `kubectl apply` creates a resource that "doesn't exist yet" and
  is documented as create or update. `kubectl create -f FILENAME` is the
  separate create form. The apply page tells you to create initially with
  either `apply` or `create --save-config`.
  <https://kubernetes.io/docs/reference/kubectl/generated/kubectl_apply/>
  <https://kubernetes.io/docs/reference/kubectl/generated/kubectl_create/>
  <https://kubernetes.io/docs/reference/kubectl/conventions/>
- cargo: "`cargo add` ... can add or modify dependencies." "When you add a
  package that is already present, the existing entry will be updated."
  <https://doc.rust-lang.org/cargo/commands/cargo-add.html>
- git config: "You can query/set/replace/unset options with this command."
  `set` is "Set value for one or more config options. By default, this
  command refuses to write multi-valued config options."
  <https://git-scm.com/docs/git-config>
- kubectl config: "Specifying an attribute name that already exists will
  merge new fields on top of existing values."
  <https://kubernetes.io/docs/reference/kubectl/generated/kubectl_config/kubectl_config_set/>
- gh: `gh config set` is "Update configuration with a value for the given
  key."
  <https://cli.github.com/manual/gh_config_set>
- npm config: `set` "Sets each of the config keys to the value provided."
  "If value is omitted, the key will be removed from your config file
  entirely."
  <https://docs.npmjs.com/cli/v11/commands/npm-config/>
- npm init: additive. It will "keep any fields and values that were already
  set." It does not replace existing fields.
  <https://docs.npmjs.com/cli/v11/commands/npm-init/>
- git init: safe to rerun and "will not overwrite things that are already
  there."
  <https://git-scm.com/docs/git-init>
- cargo new versus cargo init: `cargo new` "will create a new Cargo package
  in the given directory." The cargo-init page points at cargo-new as "a
  similar command which will create a new package in a new directory," and
  cargo-init itself creates the manifest in the current directory.
  <https://doc.rust-lang.org/cargo/commands/cargo-new.html>
  <https://doc.rust-lang.org/cargo/commands/cargo-init.html>
- REUSE annotate: if a header with copyright or licensing information
  already exists, "that header is replaced in-place." `--no-replace` keeps
  the old header. `--skip-existing` is "Skip a file if there is already
  REUSE information inside of it."
  <https://reuse.readthedocs.io/en/stable/man/reuse-annotate.html>
- REUSE download: "downloads licenses into your `LICENSES/` directory" and
  creates that directory if needed. The page does not say whether an
  existing file is overwritten.
  <https://reuse.readthedocs.io/en/stable/man/reuse-download.html>
- git apply: on a partial failure, "by default fails the whole patch and
  does not touch the working tree." `--reject` is the opt-in for a partial
  apply.
  <https://git-scm.com/docs/git-apply>

Licencify `apply` follows the kubectl and `cargo add` pattern: one verb
covers first write and later write. The confirm-before-replace rule is
stricter than those two manuals, which do not document a prompt for that
update. The "several primary candidates, change nothing" rule matches
`git apply`'s default of failing the whole operation without touching the
tree. `init` matches `git init` and additive `npm init`: do not overwrite
an existing config file.

`git apply` is a hazard only because the word is the same. The job in that
manual is a patch, not a licence file. Help text can say "create or
replace" the way the kubectl conventions page does, without renaming the
command.

## 3. How do they choose a config file from a subdirectory?

None of these manuals prompt for a destination. They pick a file with a
scope flag, a path argument, or by walking upward. Writes go to that one
file. They do not add a path-keyed section because the current directory
changed.

git discovers the repository by finding a directory with a `.git`
subdirectory. `--git-dir` "turns off the repository discovery." `-C <path>`
will "Run as if git was started in `<path>` instead of the current working
directory."
<https://git-scm.com/docs/git>
`git config` reads system, global, and repository files. "When writing, the
new value is written to the repository local configuration file by default."
`--global`, `--system`, `--local`, `--worktree`, and `--file <config-file>`
select the file. "git config will only ever change one file at a time."
<https://git-scm.com/docs/git-config>
From a subdirectory, the default write is still that repository
`.git/config`, not a section named after the subdirectory.

cargo add and cargo fetch: `--manifest-path` is the "Path to the
`Cargo.toml` file. By default, Cargo searches for the `Cargo.toml` file in
the current directory or any parent directory."
<https://doc.rust-lang.org/cargo/commands/cargo-add.html>
<https://doc.rust-lang.org/cargo/commands/cargo-fetch.html>
Cargo config files are separate files, probed from the current directory
through parents, then `$CARGO_HOME/config.toml`. "If, for example, Cargo
were invoked in `/projects/foo/bar/baz`, then the following configuration
files would be probed." Nearer files win.
<https://doc.rust-lang.org/cargo/reference/config.html>
`-C PATH` changes the working directory before the command, on nightly with
`-Z unstable-options`.
<https://doc.rust-lang.org/cargo/commands/cargo-init.html>

npm: `npm prefix` will "Print the local prefix to standard output. This is
the closest parent directory to contain a `package.json` file or
`node_modules` directory, unless `-g` is also specified."
<https://docs.npmjs.com/cli/v11/commands/npm-prefix/>
The project `.npmrc` is "in the root of the project (ie, a sibling of
`node_modules` and `package.json`)."
<https://docs.npmjs.com/cli/v11/configuring-npm/npmrc/>
`npm config set` "Modifies the user configuration file unless `location` is
passed." `location` is "`user`" unless `--global` is passed, and may be
`global`, `user`, or `project`.
<https://docs.npmjs.com/cli/v11/commands/npm-config/>
So a subdirectory still resolves one project root, and a project write
targets that root `.npmrc` only when `--location=project` (or the
equivalent) is set. The default write is the user file, not a subdirectory
section.

kubectl does not use the current directory to pick a config file. Apply
inherits `--kubeconfig` ("Path to the kubeconfig file to use for CLI
requests") and `-n, --namespace`. `kubectl config set` inherits
`--kubeconfig` ("use a particular kubeconfig file").
<https://kubernetes.io/docs/reference/kubectl/generated/kubectl_apply/>
<https://kubernetes.io/docs/reference/kubectl/generated/kubectl_config/kubectl_config_set/>

Hugo: `hugo new` says "Ensure you run this within the root directory of
your project." Inherited flags are `--config` ("config file (default is
hugo.yaml|json|toml)"), `--configDir` (default `"config"`), and `-s,
--source` ("filesystem path to read files relative from").
<https://gohugo.io/commands/hugo_new/>
<https://gohugo.io/commands/hugo/>
The pages fetched here do not say that Hugo walks parent directories.

gh config is the gh client configuration, not a file discovered from the
project directory. `gh config set` takes `-h, --host` for a per-host
setting. The documented settings include `prompt` (`enabled` or
`disabled`).
<https://cli.github.com/manual/gh_config>
<https://cli.github.com/manual/gh_config_set>
`gh repo delete` with no argument "deletes the current repository."
<https://cli.github.com/manual/gh_repo_delete>

REUSE: `--root PATH` will "Set the root of the project to `PATH`. Normally
this defaults to the root of the current working directory's VCS
repository, or to the current working directory." `reuse download` writes
into `LICENSES/` "in the root of your project."
<https://reuse.readthedocs.io/en/stable/man/reuse.html>
<https://reuse.readthedocs.io/en/stable/man/reuse-download.html>

Licencify already discovers one project root (an absolute `PRJ_ROOT`, else
the Git worktree root, else the highest ancestor with a project config,
else the current directory). That discovery matches git's walk to `.git`,
npm's walk to `package.json`, and REUSE's walk to the VCS root.
<https://github.com/MRDGH2821/Licencify/blob/main/docs/superpowers/specs/2026-09-30-licencify-product-spec.md>

The unusual part is the write target. From a subdirectory, `apply` and
`config set` write an exact-path `[[subdirs]]` entry in the root file.
`--root` selects the shared section and takes no path. Other tools either
write the one project file found by walking up (git `--local`, cargo
`--manifest-path`, npm `--location=project`) or select a different file
with `--global`, `--file`, `--kubeconfig`, or `--config`. They do not ask
which section to write. Licencify's "no destination prompt" matches that.
The cwd rule itself does not.

## 4. What do the manuals actually require for help, examples, confirmation, streams, and exit codes?

### Help

git: `-h` "Prints the synopsis and a list of the most commonly used
commands." `--help` "is converted internally into `git help`." "You can
learn more about individual Git commands with `git help command`."
<https://git-scm.com/docs/git>
gitcli: commands with the enhanced option parser treat `-h` as "a pretty
printed usage." "`git subcmd -h` without anything else on the command line
is meant to consistently give the usage." Long options can be negated with
`--no-`. "Options trump configuration and environment."
<https://git-scm.com/docs/gitcli>

gh: `gh help [command]` "provides help for any command."
<https://cli.github.com/manual/gh_help>
npm: "Run `npm help` to get a list of available commands."
<https://docs.npmjs.com/cli/v11/commands/npm/>
cargo, on each command page fetched here: `-h, --help` "Prints help
information."
<https://doc.rust-lang.org/cargo/commands/cargo-init.html>
kubectl apply, create, and config set each document `-h, --help`.
<https://kubernetes.io/docs/reference/kubectl/generated/kubectl_apply/>
Hugo documents `-h, --help` as "help for hugo."
<https://gohugo.io/commands/hugo/>
REUSE: `--help` "Display help and exit. If no command is provided, this
option is implied."
<https://reuse.readthedocs.io/en/stable/man/reuse.html>

The guidelines say to show help for `-h`, `--help`, `help`, and
`subcommand --help`, to lead with examples, and to put common commands
first. If a command "requires arguments to function, and is run with no
arguments, display concise help." They say to ignore that when the program
"is interactive by default (e.g. `npm init`)."
<https://clig.dev/>

Licencify `cache` with no subcommand, and `apply` with no id, are specified
to do work, not to require further arguments. The guideline's "show help
when arguments are missing" rule does not demand that those two forms print
help instead.

### Examples

These manuals include an examples section or an examples block on the
command page:

- `git init` and `git config` and `git add`
  <https://git-scm.com/docs/git-init>
  <https://git-scm.com/docs/git-config>
  <https://git-scm.com/docs/git-add>
- `gh config set`, including `gh config set editor vim`
  <https://cli.github.com/manual/gh_config_set>
- `cargo init`, `cargo new`, `cargo add`, `cargo fetch`, `cargo search`
  <https://doc.rust-lang.org/cargo/commands/cargo-add.html>
  <https://doc.rust-lang.org/cargo/commands/cargo-fetch.html>
- `npm init`, including `npm init -y`
  <https://docs.npmjs.com/cli/v11/commands/npm-init/>
- `kubectl apply`, `kubectl create`, and `kubectl config set`
  <https://kubernetes.io/docs/reference/kubectl/generated/kubectl_apply/>
  <https://kubernetes.io/docs/reference/kubectl/generated/kubectl_config/kubectl_config_set/>
- `reuse annotate`, with
  `reuse annotate --copyright="Jane Doe" --license=MIT my_file.py`
  <https://reuse.readthedocs.io/en/stable/man/reuse-annotate.html>

The `hugo` and `hugo new` pages fetched here show a synopsis and flags.
They do not show an examples block.
<https://gohugo.io/commands/hugo_new/>
The guidelines say "Lead with examples."
<https://clig.dev/>

### Confirmation and `--yes`

The guidelines: "Only use prompts or interactive elements if `stdin` is an
interactive terminal." If it is not, "skip prompting and just require those
flags/args" and "throw an error telling the user what flag to pass."
"Never require a prompt." A common convention is "to prompt for the user to
type `y` or `yes` if running interactively, or requiring them to pass `-f`
or `--force` otherwise." `--no-input` "don't prompt." For a moderate local
change, "You usually want to prompt for confirmation."
<https://clig.dev/>

gh implements the skip flag as `--yes`. On `gh repo delete`, `--yes` means
"Confirm deletion without prompting." For the current repository, "`--yes`
is ignored and you will be prompted." A script must name the repository:
`gh repo delete owner/repo --yes`.
<https://cli.github.com/manual/gh_repo_delete>
`gh config` can set `prompt` to `disabled`.
<https://cli.github.com/manual/gh_config>

npm: `npm init` asks questions unless `-y` / `--yes` is passed. The `yes`
config will "Automatically answer `yes` to any prompts that npm might print
on the command line." `--force` on `npm init` will "Implicitly set `--yes`
during `npm init`."
<https://docs.npmjs.com/cli/v11/commands/npm-init/>
`npm cache clean` "requires `--force` to run."
<https://docs.npmjs.com/cli/v11/commands/npm-cache/>

kubectl apply's `--force` is about immediate deletion: "immediate deletion
of some resources may result in inconsistency or data loss and requires
confirmation." The create-or-update synopsis does not document a
confirmation prompt.
<https://kubernetes.io/docs/reference/kubectl/generated/kubectl_apply/>

`git init`, `git config set`, and `cargo add` do not document a
confirmation prompt for rewrite.
<https://git-scm.com/docs/git-init>
<https://git-scm.com/docs/git-config>
<https://doc.rust-lang.org/cargo/commands/cargo-add.html>
`git add` can be interactive (`-i`, `-p`), and `-n` / `--dry-run` means
"Don't actually add the file(s)."
<https://git-scm.com/docs/git-add>
kubectl documents `--dry-run` as `none`, `server`, or `client`.
<https://kubernetes.io/docs/reference/kubectl/generated/kubectl_apply/>
The guidelines list `-n, --dry-run` as "Do not run the command, but describe
the changes that would occur."
<https://clig.dev/>

Licencify already matches the prompt-on-replace plus `--yes` pattern used
by `gh repo delete` and `npm init`, and the "scripted invocations must not
wait" rule in the product spec matches the guideline that a non-interactive
stdin must not block on a prompt.
<https://github.com/MRDGH2821/Licencify/blob/main/docs/superpowers/specs/2026-09-30-licencify-product-spec.md>

### stdout and stderr

The guidelines: "Send output to `stdout`." "Send messaging to `stderr`."
"Log messages, errors, and so on should all be sent to `stderr`." Provide
`-q` "to suppress all non-essential output."
<https://clig.dev/>

`git apply --verbose` will "Report progress to stderr." `--quiet` will
"Suppress stderr output."
<https://git-scm.com/docs/git-apply>
`npm config get` will "Echo the config value(s) to stdout."
<https://docs.npmjs.com/cli/v11/commands/npm-config/>
`npm prefix` prints the prefix "to standard output."
<https://docs.npmjs.com/cli/v11/commands/npm-prefix/>
cargo's `-q` / `--quiet` "Do not print cargo log messages."
<https://doc.rust-lang.org/cargo/commands/cargo-init.html>
Hugo's `--quiet` is "build in quiet mode."
<https://gohugo.io/commands/hugo/>
`reuse lint -q, --quiet` "Do not print anything to STDOUT." `-j, --json`
"Output the results of the lint as JSON."
<https://reuse.readthedocs.io/en/stable/man/reuse-lint.html>
`npm config list` accepts `--json`. `npm search` documents `json` and
`parseable` ("tab-separated table format").
<https://docs.npmjs.com/cli/v11/commands/npm-config/>
<https://docs.npmjs.com/cli/v11/commands/npm-search/>
kubectl conventions: "For a stable output in a script", request `-o name`,
`-o json`, `-o yaml`, `-o go-template`, or `-o jsonpath`. "Don't rely on
context, preferences, or other implicit states."
<https://kubernetes.io/docs/reference/kubectl/conventions/>
`git config` can add `--show-origin` and `--show-scope` to queried values.
<https://git-scm.com/docs/git-config>

The Licencify spec already says `apply` reports files written, `config
list` identifies loaded files and effective values, and `--verbose` reports
each setting's source without printing the value. It does not assign those
reports to stdout or stderr.
<https://github.com/MRDGH2821/Licencify/blob/main/docs/superpowers/specs/2026-09-30-licencify-product-spec.md>

### Exit codes

The guidelines: "Return zero exit code on success, non-zero on failure.
Map the non-zero exit codes to the most important failure modes."
<https://clig.dev/>

`git config`: "On success, the command returns the exit code 0." It then
maps ret=1 through ret=6 (invalid key, missing name, invalid file, cannot
write, unset or multi-match, invalid regexp). `get` "Returns error code 1
if key is not present."
<https://git-scm.com/docs/git-config>
`git add --ignore-errors` "shall still exit with non-zero status."
<https://git-scm.com/docs/git-add>

Every cargo command page fetched here says:

- `0`: Cargo succeeded.
- `101`: Cargo failed to complete.

<https://doc.rust-lang.org/cargo/commands/cargo-init.html>
<https://doc.rust-lang.org/cargo/commands/cargo-add.html>
<https://doc.rust-lang.org/cargo/commands/cargo-fetch.html>

kubectl apply documents `--warnings-as-errors`: "Treat warnings received
from the server as errors and exit with a non-zero exit code." The
conventions page fetched here does not list numeric exit codes.
<https://kubernetes.io/docs/reference/kubectl/generated/kubectl_apply/>
<https://kubernetes.io/docs/reference/kubectl/conventions/>

The gh, npm, Hugo, and REUSE pages fetched here do not publish an exit-code
table. This note does not invent one for them.

The Licencify spec already requires a nonzero result for a confirmed `scan`
conflict, and it says failures must not be reported as full success. It
does not map those cases to numbers.
<https://github.com/MRDGH2821/Licencify/blob/main/docs/superpowers/specs/2026-09-30-licencify-product-spec.md>

## 5. Improvements

These items were not adopted. The product spec leaves them out of the current target.

1. Lead `--help` with examples, as the guidelines and the git, gh, cargo,
   npm, kubectl, and REUSE pages do. For `apply`, show no id, one id, two
   ids, a subdirectory, `--root`, and `--yes`. For `config`, show `get`,
   `list`, and `set`. For `cache`, show the bare command, `update`,
   `fetch`, `fetch <id>`, and `clear`. State in the first lines that
   `apply` creates a missing primary file and confirms before replacing one
   existing primary file. That is the kubectl "create or update" wording,
   plus the settled prompt. Also say that `apply` is not `git apply`.
2. Accept `licencify help`, `licencify help <command>`, `-h`, and `--help`
   on the root and on each subcommand, matching `git help`, `gh help`, and
   the guideline's git-like list. REUSE implies `--help` when no command is
   given. Do that only for an unknown or missing command. Do not replace
   the settled no-argument actions of `cache` and `apply`.
3. Print machine-readable results on stdout and prompts, warnings, and
   progress on stderr. `npm config get` and the guidelines put values on
   stdout. `git apply --verbose` puts progress on stderr. The replace
   prompt therefore belongs on stderr. The file list, `config get` value,
   and `cache` directory line belong on stdout.
4. When stdin is not a terminal and a primary file would be replaced, do
   not wait. Exit nonzero and name `--yes` in the error. The spec already
   says a script must not wait. The guidelines say the error should tell
   the user which flag to pass. Interactive prompts can accept `y` or
   `yes`, which is the guideline's wording. `--yes` stays the skip flag
   because `gh repo delete` and `npm init` document that name.
5. Document exit status in the help text. Use `0` for success. Use nonzero
   for failure, and reserve distinct codes for the failures the spec
   already names: repeated id, `proprietary` mixed with another id, several
   primary candidates, missing required licence or author, and a confirmed
   `scan` conflict. `git config` publishes this kind of table. Cargo
   publishes `0` and `101`. Publishing the table does not require Cargo's
   `101`.
6. Teach the subdirectory write rule in help, the way git documents
   `--local` versus `--global` and cargo documents `--manifest-path`. Show
   that the current directory selects the shared section or one
   `[[subdirs]]` path, and that `--root` writes the shared section and does
   not take a path. `config list` already "identifies loaded files and
   effective values." Print the winning file beside each value, in the
   spirit of `git config --show-origin`.
7. Add a script output switch already spelled in these manuals: `--json`
   on `config list`, `list`, `scan`, and bare `cache` (`npm config list
   --json`, `reuse lint --json`, kubectl `-o json`). Add `--quiet` for
   non-essential stderr (`git apply --quiet`, cargo `-q`, `hugo --quiet`,
   `reuse lint --quiet`). `--quiet` does not skip the replace prompt.
   `--yes` does.
8. Add `--dry-run` on `apply` so it prints the files and config writes it
   would make and then changes nothing. `git add -n`, kubectl `--dry-run`,
   and the guidelines all document that pattern. `cargo add` also documents
   `--dry-run` as "Don't actually write the manifest."
   <https://doc.rust-lang.org/cargo/commands/cargo-add.html>

### Would reopen a settled decision

Do not do these. They undo an ADR or a decided spec item.

- Split `apply` back into `add` and `update`, or rename `apply` to `add` or
  `set`.
  <https://github.com/MRDGH2821/Licencify/blob/main/docs/adr/0016-project-licences-are-applied.md>
- Add `config show`, `config init`, or a `schema` subcommand.
  <https://github.com/MRDGH2821/Licencify/blob/main/docs/adr/0013-config-get-list-set.md>
  <https://github.com/MRDGH2821/Licencify/blob/main/docs/adr/0015-config-schema-is-a-cargo-example.md>
- Add `cache info`, or `--update` / `--fetch` flags on `cache`.
  <https://github.com/MRDGH2821/Licencify/blob/main/docs/adr/0014-cache-update-and-fetch.md>
- Make bare `cache` print help instead of the directory, count, and size.
- Make `cache fetch <id>` refetch a file that is already cached.
- Add `remove` or `delete`. The spec says deleting a licence file is out of
  scope. `git-rm` shows that git has the verb. It does not require a
  licence tool to delete files.
  <https://github.com/MRDGH2821/Licencify/blob/main/docs/superpowers/specs/2026-09-30-licencify-product-spec.md>
  <https://git-scm.com/docs/git>
- Prompt for shared versus subdirectory, or always write the shared section
  from a subdirectory. Both were considered and rejected.
  <https://github.com/MRDGH2821/Licencify/blob/main/docs/adr/0016-project-licences-are-applied.md>
- Rename `--root` or `--yes`.
- Rename `detect` and `scan` to `lint` and `lint-file`.

## Conflicts

No cited manual requires a different verb for `init`, `list`, `search`,
`config get`, `config list`, or `config set`. No cited manual requires a
destination prompt, a `schema` subcommand, `cache info`, or a `remove`
command for this job. kubectl's manual uses `apply` for create-or-update,
so that verb is not in conflict. `git apply` is a different job (a patch),
which is a reading hazard, not a requirement to rename.

One flag-name collision, not a required reversal: REUSE's `--root PATH`
will "Set the root of the project to `PATH`." Licencify's settled `--root`
takes no
path and writes the shared config section.
<https://reuse.readthedocs.io/en/stable/man/reuse.html>
<https://github.com/MRDGH2821/Licencify/blob/main/docs/adr/0016-project-licences-are-applied.md>
REUSE does not specify Licencify's flags. Renaming `--root` would reopen
ADR 0016. The compatible fix is the help sentence in improvement 6: define
`--root` before any example uses it.

The guidelines mention `-f` / `--force` and `--no-input` for skipping
prompts. They do not forbid `--yes`. gh and npm document `--yes`, so the
settled flag is not in conflict with the manuals that actually implement a
skip flag.
<https://clig.dev/>
<https://cli.github.com/manual/gh_repo_delete>
<https://docs.npmjs.com/cli/v11/commands/npm-init/>
