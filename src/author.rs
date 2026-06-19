use anyhow::Result;

use crate::config::Config;
use crate::process::Runner;

/// Strategy for resolving a copyright author name.
pub trait AuthorResolver {
    /// Attempt to resolve the author. Returns `None` if this resolver
    /// cannot provide a value (caller tries the next resolver).
    fn resolve(&self, config: Option<&Config>) -> Option<Result<String>>;
}

/// Resolve author from `git config user.name`.
pub struct GitAuthorResolver<'a> {
    pub runner: &'a dyn Runner,
}

impl AuthorResolver for GitAuthorResolver<'_> {
    fn resolve(&self, _config: Option<&Config>) -> Option<Result<String>> {
        let output = self.runner.run_command("git", &["config", "user.name"])?;

        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return Some(Ok(name));
            }
        }
        None
    }
}

/// Resolve author from the licencify config file (`[default].author` field).
pub struct ConfigAuthorResolver;

impl AuthorResolver for ConfigAuthorResolver {
    fn resolve(&self, config: Option<&Config>) -> Option<Result<String>> {
        let cfg = config?;
        let author = cfg.default.author.as_deref()?;
        if author.trim().is_empty() {
            None
        } else {
            Some(Ok(author.to_string()))
        }
    }
}

/// Resolve author using CLI arg, then resolvers in order.
/// First `Some` result wins.
pub fn resolve_author(
    cli_author: Option<String>,
    config: Option<&Config>,
    resolvers: &[&dyn AuthorResolver],
) -> Result<String> {
    if let Some(author) = cli_author {
        return Ok(author);
    }
    for resolver in resolvers {
        if let Some(result) = resolver.resolve(config) {
            return result;
        }
    }
    anyhow::bail!(
        "No author specified. Set one via `licencify config --author <name>` \
         or configure git with `git config user.name \"Your Name\"`"
    )
}
