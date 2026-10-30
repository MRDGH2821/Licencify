# Project config uses the PRJ directory

An early plan stored project settings in a root `licencify.toml` and wrote `config set` only to the user file. That split version-controlled settings from the command that edits them. Project settings live in the PRJ config directory. `config set` writes the shared project file, or an exact-path subdir entry when run from a subdirectory. The product spec has the full precedence.
