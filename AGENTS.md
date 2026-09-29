# AGENTS Instructions

This file provides guidance for AI coding assistants working with this project.

## MANDATORY: AI Co-authored-by Trailer

> **Every commit made with AI assistance MUST include a `Co-authored-by` trailer. No exceptions.**

**Format:**

```txt
Co-authored-by: <Model Name> via <Tool> <noreply@provider-domain>
```

**Provider noreply addresses:**

<!-- smt -->

| Provider                | noreply address          |
| ----------------------- | ------------------------ |
| Anthropic (Claude)      | `noreply@anthropic.com`  |
| Cursor                  | `cursoragent@cursor.com` |
| Google (Gemini)         | `noreply@google.com`     |
| Meta (Llama)            | `noreply@meta.com`       |
| Microsoft (Copilot)     | `noreply@microsoft.com`  |
| Mistral                 | `noreply@mistral.ai`     |
| OpenAI (GPT / o-series) | `noreply@openai.com`     |
| xAI (Grok)              | `noreply@x.ai`           |

**Examples:**

```txt
feat(pre-commit): add spell checking to commit messages

Co-authored-by: Claude Sonnet 4.6 via opencode <noreply@anthropic.com>
```

```txt
fix(cspell): resolve configuration issue

Co-authored-by: GPT-4o via Cursor <noreply@openai.com>
```

**Rules:**

- Use the **exact model name and version** you are running as (e.g. `Claude Sonnet 4.6`, not just `Claude`)
- Use the **tool name** as it is commonly known (e.g. `opencode`, `Cursor`, `Copilot`, `Zed`)
- If the model version is unknown, use the model family name (e.g. `Claude Sonnet`)
- One trailer per AI model involved
- **Never omit this trailer** when the commit was AI-assisted — this is how git history stays honest

## Setup: skills and MCP

Before substantive work, ensure project skills and MCP servers are installed.

1. From the repository root, run `mise run ai-setup`, or:

   ```sh
   capa install
   ```

2. **Reload the agent** (new chat / restart the agent session) so installed skills and MCP servers are picked up.

Configuration lives in `capabilities.yaml`. Do not skip this when skills or MCP tools are missing or stale.

## Project Context

- **Project Type**: Project scaffolded from [copier-mr-mise](https://github.com/MRDGH2821/copier-mr-mise)
- **Key Technologies**: mise, hk, MegaLinter, cspell, capa
- **Purpose**: Standardized starting point with tool management, git hooks, and quality checks
- **Template updates**: `copier update` (review scripts in the template's `copier.yml`)

## Layout

| Path                   | Purpose                                             |
| ---------------------- | --------------------------------------------------- |
| `mise.toml`            | Tools, tasks, `hk install --mise` postinstall hook  |
| `.config/hk.pkl`       | hk hook config (pre-commit, commit-msg, fix, check) |
| `capabilities.yaml`    | Skills, MCP servers, and providers (capa)           |
| `cog.toml`             | Conventional-commit scopes and version bump hooks   |
| `.mega-linter.yml`     | MegaLinter config                                   |
| `.config/treefmt.toml` | Full-tree formatter                                 |
| `.config/cspell.json`  | Spell-check dictionary                              |
| `.agents/logs/`        | AI-assisted work logs                               |

## Recommendations

### Configuration directory

If this project is a tool, CLI, or library that reads its own configuration,
make it resolve that configuration from a project-level `.config/` directory
(e.g. `.config/<project-name>.toml`) in addition to any other supported
locations. This template already keeps its own tool configs there
(`.config/cspell.json`, `.config/rumdl.toml`, `.config/cliff.toml`); extending
the same convention to what this project ships keeps consumers' repo roots tidy.

References:

- <https://github.com/numtide/prj-spec> — project directory specification
- <https://dot-config.github.io/> — the `.config/` directory convention
- <https://github.com/pi0/config-dir> — reference implementation for resolving it

## Branch naming strategy

Since many people will be contributing to this repository, we use a branching strategy that allows for parallel development while keeping the main branch stable.

Use the following branching strategy:

`<human first name>/<work type>/<work name>`

For example:

- `john/feat/add-packages`
- `jane/fix/ui-bugs`
- `joy/refactor/payment`

`<human first name>` - will be derived from `git config user.name` or the author's first name. Ask the author for their first name if it's not available.
`<work type>` - the type of work being done (e.g., `feat`, `fix`, `refactor`). Should match commit types from conventional commits.
`<work name>` - the name of the work being done (e.g., `add-packages`, `ui-bugs`, `payment`)

## General Guidelines

### Communication

- Explain what you're doing and why before making changes
- Ask for clarification when requirements are ambiguous
- Provide context for decisions, especially when multiple approaches exist

### Code Quality

- Follow existing code style and conventions in the project
- Run linters and formatters before committing changes
- Ensure all changes pass git hooks (`hk run pre-commit`)

### File Operations

- Always check if a file exists before attempting to modify it
- Use appropriate tools to search for files rather than guessing paths
- Preserve file formatting and structure unless explicitly asked to change it

### AI-Assisted Work Documentation

- Document all AI-assisted changes in the `.agents/logs` folder as markdown files
- Use the naming format: `YYYY-MM-DD.md` (e.g., `2024-12-15.md`)
- Each documentation file should include:
  - The prompt or request that initiated the work
  - The author of the prompt (can be obtained from `git config user.name` or by asking the user)
  - Description of what was done
  - Which AI model was used (e.g., Claude Sonnet 4.5, GPT-4, etc.)
- If more prompts are provided on the same day, append them to the existing log file with timestamps
- Use the `date` command to generate timestamps (e.g., `date --iso-8601=seconds` or `date '+%Y-%m-%d %H:%M:%S'`)
- Place any other relevant documents (prompts, examples, references) in the `.agents` folder
- This provides transparency and helps track AI contributions to the project

## Dev Environment Tips

- Use `--help` or `help` subcommand to get help on a command. It can even reveal hints on how to proceed ahead or optimize the number of steps.
- Check tool documentation before asking the user for configuration details
- Tools are managed by **mise**. Prefer `mise run <task>` over ad-hoc binaries when a task exists.

## Tooling

### mise & hk

Use the configured mise mcp server. If mise's mcp tools are not available, tell the user to fix by referring the following:

- For mise - <https://mise.jdx.dev/mcp.html>
- For hk - <https://hk.jdx.dev/agents.html#mcp>

### Using hk from a coding agent

Inspect and plan before running. Scope checks to changed files with `--files0-from` and use `--cd` to select the project root. Prefer `--safe`, inspect command effects, and require approval for unknown or destructive commands.

Consume JSON or JSONL diagnostics while retaining raw output, and always review the diff produced by a fix.

MCP clients should use `inspect_project`, `plan`, safe run tools, paged output, and `get_diff` rather than invoking arbitrary shell commands.

### MegaLinter

- Config: `.mega-linter.yml`
- Use the MegaLinter skill when it is installed
- Reports: `megalinter-reports/`
- Not all linters need to pass — some are informational

### CSpell

- Config: `.config/cspell.json`
- Add project-specific words to the `words` array
- Don't disable spell checking without good reason
- Run with `mise run cspell`

### Formatting and Hooks (hk)

- Run `hk run fix` or `mise run fmt` before committing to format all supported file types
- `hk` integrates formatters and linters in `.config/hk.pkl` for staged files and hook checks

## Commit Messages

### Format

- Follow Conventional Commits format: `<type>(<scope>): <description>` as given here - <https://www.conventionalcommits.org/en/v1.0.0/>
- Valid types: `build`, `chore`, `ci`, `docs`, `feat`, `fix`, `perf`, `refactor`, `revert`, `style`, `test`
- For valid scopes, refer to the `scopes` array in `cog.toml` — it is the source of truth.

### Examples

```txt
feat(pre-commit): add spell checking to commit messages
fix(cspell): resolve configuration issue
docs: update AGENTS.md with guidelines
chore(cspell): add technical terms to dictionary
```

Version bumps use cocogitto (`cog bump`); pre-bump hooks update `package.json` and `CHANGELOG.md` (git-cliff).

## Troubleshooting

### Common Issues

**Git hooks failing on commit:**

- Read the error message — it usually points directly to the fix
- Try to fix the issue and retry the commit; do not skip hooks
- Fix formatting first (`hk run fix` or `mise run fmt`)
- Then address spell checking and linting

**Spell check failures:**

- Add legitimate technical terms to `.config/cspell.json` `words` array
- Use proper capitalization for proper nouns
- Don't add obvious typos to the dictionary

### Getting Help

- Review existing configuration files for examples

## Best Practices

### Before Making Changes

1. Understand the current state of the project
2. Check if similar functionality already exists
3. Review relevant configuration files

### When Adding Dependencies

- Prefer tools that don't require heavy installation; add them via `mise.toml` when they should be shared
- Document installation steps clearly
- Consider cross-platform compatibility
- Update relevant configuration files

### Testing Changes

- Verify the project structure is correct
- Ensure documentation is updated
