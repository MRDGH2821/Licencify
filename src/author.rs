use anyhow::Result;

/// Strategy for resolving a copyright author name.
pub trait AuthorResolver {
    /// Attempt to resolve the author. Returns `None` if this resolver
    /// cannot provide a value (caller tries the next resolver).
    fn resolve(&self) -> Option<Result<String>>;
}

/// Resolve author from `git config user.name`.
pub struct GitAuthorResolver;

impl AuthorResolver for GitAuthorResolver {
    fn resolve(&self) -> Option<Result<String>> {
        let output = std::process::Command::new("git")
            .args(["config", "user.name"])
            .output()
            .ok()?;

        if output.status.success() {
            let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !name.is_empty() {
                return Some(Ok(name));
            }
        }
        None
    }
}

/// Resolve author from the licencify config file (`default_author` field).
pub struct ConfigAuthorResolver;

impl AuthorResolver for ConfigAuthorResolver {
    fn resolve(&self) -> Option<Result<String>> {
        let cfg = crate::config::Config::load().ok()?;
        let author = cfg.default_author?;
        if author.trim().is_empty() {
            None
        } else {
            Some(Ok(author))
        }
    }
}

/// Resolve author using CLI arg, then resolvers in order.
/// First `Some` result wins.
pub fn resolve_author(
    cli_author: Option<String>,
    resolvers: &[&dyn AuthorResolver],
) -> Result<String> {
    if let Some(author) = cli_author {
        return Ok(author);
    }
    for resolver in resolvers {
        if let Some(result) = resolver.resolve() {
            return result;
        }
    }
    anyhow::bail!(
        "No author specified. Set one via `licencify config --author <name>` \
         or configure git with `git config user.name \"Your Name\"`"
    )
}
