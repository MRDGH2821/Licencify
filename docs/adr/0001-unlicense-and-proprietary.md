# Unlicense and proprietary are different

`unlicence` and `unlicense` both mean The Unlicense (<https://unlicense.org/>). Config and manifests store the SPDX id `Unlicense`, because a manifest has to be valid SPDX. `proprietary` is Licencify's all-rights-reserved notice, not that dedication.

`UNLICENSED` is neither: it is not an SPDX id, and accepting it as `proprietary` made the two look like one licence. Input `UNLICENSED` is rejected. Removing that alias from the current code is deferred; until then the program still treats `UNLICENSED` as `proprietary`.

## Considered options

- Keep today's alias, where `UNLICENSED` selects `proprietary` and detection reports `UNLICENSED`.
- Write the string `UNLICENSED` into npm manifests for a proprietary notice.
- Store a second id spelled `unlicence`.
