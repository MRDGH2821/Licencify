# Cache uses update, fetch, and a bare command

`cache update` refreshes `licenses.json` only. `cache fetch` downloads the per-licence detail JSON. `licencify cache` with no subcommand prints the cache directory, the file count, and the size. Those used to be `update-spdx-index`, `cache fetch-all`, and `cache info`.

## Considered options

- Keep the long names `update-spdx-index` and `cache fetch-all`.
- Use flags `--update` and `--fetch` on `cache`. Verb flags were rejected. Git retired `git config --get` in favour of `git config get`.
- Keep a `cache info` subcommand for the directory, count, and size.
