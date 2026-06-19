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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    use std::process::Output;

    /// Mock runner that returns configurable output.
    struct MockRunner {
        output: Option<Output>,
    }

    impl Runner for MockRunner {
        fn run_command(&self, _program: &str, _args: &[&str]) -> Option<Output> {
            self.output.clone()
        }

        fn exit(&self, _code: i32) -> ! {
            panic!("exit called in test");
        }
    }

    #[test]
    fn cli_author_takes_precedence() {
        let resolvers: Vec<&dyn AuthorResolver> = vec![];
        let result = resolve_author(Some("CLI Author".into()), None, &resolvers).unwrap();
        assert_eq!(result, "CLI Author");
    }

    #[test]
    fn git_author_resolver_works() {
        let mock = MockRunner {
            output: Some(Output {
                status: std::process::ExitStatus::from_raw(0),
                stdout: b"Git Author\n".to_vec(),
                stderr: Vec::new(),
            }),
        };
        let resolver = GitAuthorResolver { runner: &mock };
        let result = resolver.resolve(None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().unwrap(), "Git Author");
    }

    #[test]
    fn git_author_resolver_returns_none_on_failure() {
        let mock = MockRunner { output: None };
        let resolver = GitAuthorResolver { runner: &mock };
        let result = resolver.resolve(None);
        assert!(result.is_none());
    }

    #[test]
    fn config_author_resolver_works() {
        let config = Config {
            default: crate::config::DefaultConfig {
                author: Some("Config Author".into()),
                ..Default::default()
            },
            template: None,
            subdirs: None,
        };
        let resolver = ConfigAuthorResolver;
        let result = resolver.resolve(Some(&config));
        assert!(result.is_some());
        assert_eq!(result.unwrap().unwrap(), "Config Author");
    }

    #[test]
    fn config_author_resolver_returns_none_when_no_config() {
        let resolver = ConfigAuthorResolver;
        let result = resolver.resolve(None);
        assert!(result.is_none());
    }

    #[test]
    fn config_author_resolver_returns_none_when_empty() {
        let config = Config {
            default: crate::config::DefaultConfig {
                author: Some("   ".into()),
                ..Default::default()
            },
            template: None,
            subdirs: None,
        };
        let resolver = ConfigAuthorResolver;
        let result = resolver.resolve(Some(&config));
        assert!(result.is_none());
    }

    #[test]
    fn resolve_author_chain_falls_through_to_error() {
        let resolvers: Vec<&dyn AuthorResolver> = vec![];
        let result = resolve_author(None, None, &resolvers);
        assert!(result.is_err());
    }
}
