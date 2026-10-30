# Architecture decision records

Decisions live here. Each file says what was chosen and why. The open task list is still [the implementation plan](../superpowers/plans/2026-09-30-licencify-product.md). The target behavior is [the product spec](../superpowers/specs/2026-09-30-licencify-product-spec.md).

| ADR                                                        | Decision                                                              |
| ---------------------------------------------------------- | --------------------------------------------------------------------- |
| [0001](0001-unlicense-and-proprietary.md)                  | `Unlicense` and `proprietary` are different. `UNLICENSED` is neither. |
| [0002](0002-release-binaries-named-licencify.md)           | Release binaries are named `licencify`.                               |
| [0003](0003-apache-detection-needs-both-phrases.md)        | Apache detection needs an Apache phrase and a version phrase.         |
| [0004](0004-cargo-audit-in-ci.md)                          | `cargo audit` runs before release.                                    |
| [0005](0005-command-tests-cover-the-cli.md)                | Command tests cover the CLI.                                          |
| [0006](0006-readme-documents-the-cli.md)                   | The README documents install and the commands.                        |
| [0007](0007-spdx-replaces-the-github-licence-api.md)       | SPDX replaces the GitHub licence API.                                 |
| [0008](0008-bundled-templates-are-the-offline-fallback.md) | Bundled templates are the offline fallback.                           |
| [0009](0009-project-config-uses-the-prj-directory.md)      | Project config uses the PRJ directory.                                |
| [0010](0010-licence-text-follows-spdx-wording.md)          | Licence text follows SPDX wording.                                    |
| [0011](0011-readme-updates-are-opt-in.md)                  | README updates are opt-in.                                            |
| [0012](0012-three-audit-findings-were-rejected.md)         | Three audit findings were rejected.                                   |

0001 records the proprietary plan. 0007 records the first CLI design and the SPDX analysis. 0008 records the remaining-features plan.
