# Changelog

All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

---

## [0.1.0] - 2026-06-19

### Bug Fixes

- ensure output file is licence.txt or licence.html - ([b544357](/commit/b5443577a6aeffcfa680c5491f5ad21f74b3168c)) - MRDGH2821
- standardize output filename to LICENCE.txt or LICENCE.html - ([d5f6b46](/commit/d5f6b46471865f4d9d8d2582e637b3db6bc03da7)) - MRDGH2821
- remove platform-specific error message for cache directory - ([30feec7](/commit/30feec77e89cdc7ef45f97257a3059e8d1157379)) - MRDGH2821
- replace SPDX-style <year>/<copyright holders> placeholders - ([b16605c](/commit/b16605c6548c452e50d351d291d84a889d496557)) - MRDGH2821
- also replace HTML-encoded &lt;year&gt; placeholders - ([c4c9090](/commit/c4c9090ce9ffdd2bc3107eb2ef6c7dd41c68c65d)) - MRDGH2821
- correct MemFs read_dir/remove_dir_all/create_dir_all semantics - ([17673e5](/commit/17673e50085e697e6e2ebebd25bf4c1c70ceeb57)) - MRDGH2821

### Documentation

- add plan - ([bd06939](/commit/bd06939fbf4fd8bba3bb7b04ce37d6e789b887a6)) - MRDGH2821

### Features

- integrate SPDX license registry (727 licenses) - ([3e13323](/commit/3e13323d9439b498a056c39c9a2c2755a95c7459)) - MRDGH2821
- implement CLI interface with subcommands - ([777ed5f](/commit/777ed5f170369c38a8e05a672a9cdf331036c7e3)) - MRDGH2821
- complete remaining features — builtins, templates, cache, project integration, config - ([75b52cd](/commit/75b52cd7e9fe26d5c047b3f37f21bda211be3adc)) - MRDGH2821
- add licensify as CLI alias - ([61c5eda](/commit/61c5edac3376bd92e2f89478019fca5f7f2de3a3)) - MRDGH2821
- add cache fetch-all and fix registry cache directory collision - ([1ce9d1e](/commit/1ce9d1e4fe72fc28a2f8ef2363065d469dd6f4d3)) - MRDGH2821
- add --format flag for txt/html license output - ([a8c5df2](/commit/a8c5df2fcdb110c070c92b96a49d1436d0a8b324)) - MRDGH2821
- add licence - ([84626d9](/commit/84626d978a6aa7d5dea6f014762528328fe68038)) - MRDGH2821
- add config file support with CLI commands - ([cffa168](/commit/cffa16893e74dee36fba2f662f564c130fa87914)) - MRDGH2821
- add JSON schema generation for config file - ([b0768ed](/commit/b0768ed20cad80c06fef83b4eee1c4b312a135ff)) - MRDGH2821
- add config file support with CLI commands - ([54f8698](/commit/54f8698acad99a58b0eb80098b47e9ab68643802)) - MRDGH2821
- add template.paths config option for custom templates - ([10e5149](/commit/10e51493796306e5143fcceeea4f39ff6ab52567)) - MRDGH2821
- add licence_name config for LICENCE vs LICENSE spelling - ([2183e5a](/commit/2183e5a83c1ca3523c9c3a909a2a433fe442da33)) - MRDGH2821
- add global + project-level config with sub-directory licensing - ([19160d6](/commit/19160d611b60d754e0ae682e46b1275613859739)) - MRDGH2821
- also support .licencify.toml as project config - ([d2ed91f](/commit/d2ed91f499efee77501e4579345f0a94c150d810)) - MRDGH2821
- [**breaking**]remove `config get`/`set` subcommands - ([bbf2fa2](/commit/bbf2fa262ea89a26520e48129e68b62a2f27f5c5)) - MRDGH2821
- also update project config when doing update - ([fc33d06](/commit/fc33d06fb928afd585580a191d8ae7d44a17d252)) - MRDGH2821

### Miscellaneous Chores

- **(cocogitto)** add cargo version bumping command - ([12dc4e1](/commit/12dc4e1af76638d510b1c7d07b22ea2a196d5f26)) - MRDGH2821
- **(copier)** initialise template - ([76329da](/commit/76329da693f0e3a3a746bc48bb5f6c33b897876a)) - MRDGH2821
- init cargo - ([377049f](/commit/377049f8d9c0150f192f93da3846899bf38172e2)) - MRDGH2821
- add rust formatters & linters - ([a690fca](/commit/a690fcab913063834d668ae1907cbc0f06cc0a81)) - MRDGH2821
- add shadcn/improve skill - ([0ba1fc1](/commit/0ba1fc107a2a6ac0fe6a2adf478a8ee304b40e1a)) - MRDGH2821
- gitignore generated config files - ([0df8871](/commit/0df8871a6007a16f1f7a019354a1afe73b899d0f)) - MRDGH2821
- remove dead code, add comprehensive tests, add release workflow - ([0db442c](/commit/0db442ceaa311e38dff94e093bccd5b7922cffa9)) - MRDGH2821
- set versions to 0.0.0 before bump - ([2281a21](/commit/2281a21cc23a6a2300762c1237f6238a2d5ab259)) - MRDGH2821

### Refactoring

- use Tera templates for license rendering - ([5b49a48](/commit/5b49a48b0a01826621a2795cbe331eb110eb5c34)) - MRDGH2821
- convert builtin licence strings to Tera templates - ([af62992](/commit/af62992043b2d5e228fae8150ca1e374ce40d9af)) - MRDGH2821
- remove licensify alias and sort license lists alphabetically - ([3167480](/commit/31674805222705dd8b7658b607c64b49c166b51e)) - MRDGH2821
- collapse god module and deepen codebase architecture - ([ae4b65f](/commit/ae4b65f41973c459fd9f1505db898f3406bfc5d4)) - MRDGH2821
- remove templates cache, use API cache as single source of truth - ([4926900](/commit/49269006ccaddd5dbb7e6c55150eff6351be95bc)) - MRDGH2821
- reorder license fetching to cache → API → built-ins - ([be03724](/commit/be0372449cb18332d41ce3b4ce5fe55cf1b78634)) - MRDGH2821
- nest config under [default] section - ([ec21388](/commit/ec2138830cd2174b993095c31d26d8158d095fff)) - MRDGH2821
- move template.paths to top-level config section - ([dce8220](/commit/dce82205642b05caed85f8b7f8a065ab3d33f01d)) - MRDGH2821
- make [template] config section optional - ([3b19b24](/commit/3b19b246ebc3a96b96c76228d06fbcd8bf17ff2d)) - MRDGH2821
- [**breaking**]rename schema file to licencify-schema.json - ([827c736](/commit/827c736e5ed76ff3db9aa0b6e598ae77236b9bb4)) - MRDGH2821
- prefer .licencify.toml over licencify.toml as project config - ([72b45b2](/commit/72b45b2b8342a1ca6e0f726287cb23e91576b333)) - MRDGH2821
- deepen architecture with testable seams and consolidate duplicated logic - ([c734820](/commit/c73482080f7970d128cc628607b3823cc0b607fb)) - MRDGH2821
- complete Fs trait adoption across manifest handlers and cache - ([69d4ff3](/commit/69d4ff34cfc98ab66522053065db8564409f13ec)) - MRDGH2821
- extract shared add/update setup into resolution module - ([5c87290](/commit/5c87290c18e607353806bc9c519367ecec3d995d)) - MRDGH2821
- [**breaking**]change subdirs from HashMap to array-of-tables format - ([6defa51](/commit/6defa51f6b3c0b19988bc432cb06232656087c83)) - MRDGH2821

### Style

- format files - ([3cc2a17](/commit/3cc2a176ed29cac7c12c410094378d42e1e84967)) - MRDGH2821

<!-- generated by git-cliff -->
