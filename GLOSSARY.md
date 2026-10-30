# Licencify

A command-line tool that selects a licence or a proprietary notice for a project.

## Language

**Custom template**:
A licence wording file stored on the machine, in the user config directory or the project config directory.
_Avoid_: remote registry, GitHub licence API

**Manifest update**:
A change to a project's package metadata that records the selected licence.
_Avoid_: licence file

**Proprietary**:
A Licencify notice that the work is not under an open licence.
_Avoid_: Unlicense, unlicence, unlicense, UNLICENSED

**Unlicense**:
The public-domain dedication published at unlicense.org. `unlicence` and `unlicense` name this same licence. The stored identifier is `Unlicense`.
_Avoid_: proprietary, UNLICENSED