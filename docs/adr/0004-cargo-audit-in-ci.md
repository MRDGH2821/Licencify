# Cargo audit runs before release

CI runs `cargo audit` because the tool downloads licence data and a known-bad dependency should fail before release. A local audit task runs the same check.
