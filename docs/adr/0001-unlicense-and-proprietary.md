# Unlicense and proprietary are different

`unlicence` and `unlicense` both mean The Unlicense (<https://unlicense.org/>). Config and manifests store the SPDX id `Unlicense`, because a manifest has to be valid SPDX. `proprietary` is Licencify's all-rights-reserved notice, not that dedication.

`UNLICENSED` is neither: it is not an SPDX id, and treating it as `proprietary` made the two look like one licence. Input `UNLICENSED` is rejected.

## Considered options

- Keep today's alias, where `UNLICENSED` selects `proprietary` and detection reports `UNLICENSED`.
- Write the string `UNLICENSED` into npm manifests for a proprietary notice.
- Store a second id spelled `unlicence`.
