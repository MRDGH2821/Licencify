# Missing author and email come from Git

When the command and the config omit `author` or `email`, Licencify uses `git config user.name` and `git config user.email`. It tells the user which value it filled from Git. A missing Git name is an error. A missing Git email leaves `email` empty.

## Considered options

- Leave `email` empty and do not read Git for it.
- Fill both values from Git and do not tell the user.
