# The primary filename key is basename

The config key is `basename`. The values are `licence`, `license`, and `copying`, matched without case, and written as `LICENCE`, `LICENSE`, or `COPYING` plus the format extension. Unset, the name follows the detected locale, which is `LICENCE` or `LICENSE`. `licence_file_name` and `licence_name` are rejected.

## Considered options

- `licence_file_name` or `licence-file-name`. Neither name covers `COPYING`.
- `filename`, which sounds like a path such as `LICENSE.txt`.
