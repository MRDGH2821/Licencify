# Cache uses update, fetch, and a bare command

`cache update` refreshes `licenses.json` only. `cache fetch` with no id downloads every uncached detail JSON. `cache fetch <id>` downloads that one id and does not refetch a copy already in the cache. Neither form refreshes `licenses.json`. `licencify cache` with no subcommand prints the cache directory, the file count, and the size.

## Considered options

- Keep the long names `update-spdx-index` and `cache fetch-all`.
- Use flags `--update` and `--fetch` on `cache`. Verb flags were rejected. Git retired `git config --get` in favour of `git config get`.
- Keep a `cache info` subcommand for the directory, count, and size.
- `cache fetch` always downloads every uncached detail and takes no id.
