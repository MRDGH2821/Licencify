# Plans

Open this file first. Current product work is the spec and its implementation plan.
Older plans are kept here as sources. Do not implement an old plan when the spec
disagrees with it.

## Current

| Doc                                                                         | Role                                                               |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------ |
| [Product spec](../superpowers/specs/2026-09-30-licencify-product-spec.md)   | Target behavior. Open decisions at the bottom are still undecided. |
| [Implementation plan](../superpowers/plans/2026-09-30-licencify-product.md) | Task slices for that spec. Checkboxes are still open.              |

## Done

Recorded against commit `90eb38429098d91d870af5bc05a926723893ae14`.

| Plan                                | Title                                                    | Status                     |
| ----------------------------------- | -------------------------------------------------------- | -------------------------- |
| [001](001-fix-release-workflow.md)  | Fix release workflow binary names (`smt` to `licencify`) | Done (`b6e7c13`)           |
| [002](002-fix-apache-detection.md)  | Fix Apache-2.0 detection false positive                  | Done (`44bd08b`)           |
| [003](003-add-cargo-audit.md)       | Add `cargo audit` to CI and justfile                     | Done (`b6e7c13`, with 001) |
| [004](004-add-integration-tests.md) | Add integration tests for CLI commands                   | Done (`fea3ecc`)           |
| [005](005-write-readme.md)          | Write project README                                     | Done (`7c6d6b4`)           |

003 touches the same CI file as 001. They shipped together.

## Folded into the spec

These proposed behavior. The spec is the authority. Their implementation status
was not re-checked against the current code.

| Plan                                                                         | Title                               |
| ---------------------------------------------------------------------------- | ----------------------------------- |
| [CLI design](2026-06-18_180000-licencify-cli-design.md)                      | Rust CLI implementation plan        |
| [SPDX analysis](2026-06-18_182000-spdx-integration-analysis.md)              | SPDX-first resolution               |
| [Remaining features](2026-06-18_183503-remaining-features.md)                | Templates, cache, manifests, config |
| [Global and project config](2026-06-19_0001-global-project-config-plan.md)   | Layered config and subdir rules     |
| [Proprietary](2026-06-19_164138-proprietary-no-licence.md)                   | Proprietary notice and `--no-file`  |
| [HTML and text templates](2026-06-19_170000-licence-html-format-research.md) | Template alignment with SPDX        |
| [README badge](2026-06-30_162000-readme-license-badge.md)                    | README licence section update       |

## Rejected when 001–005 were written

- Subdir-prefix matching: the prefix check requires the path separator, so `src/foobar` does not match `src/foo`.
- A thicker `Runner` trait: the existing trait is enough for git author lookup.
- Missing built-in templates such as BSL-1.0 and Zlib: the SPDX API covers them.
