use std::collections::HashMap;
use std::sync::LazyLock;

static TEMPLATES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m: HashMap<&'static str, &'static str> = HashMap::new();
    m.insert("mit", include_str!("../templates/licence/mit.tera"));
    m.insert(
        "apache-2.0",
        include_str!("../templates/licence/apache-2.0.tera"),
    );
    m.insert(
        "gpl-3.0-only",
        include_str!("../templates/licence/gpl-3.0-only.tera"),
    );
    m.insert(
        "gpl-2.0-only",
        include_str!("../templates/licence/gpl-2.0-only.tera"),
    );
    m.insert(
        "agpl-3.0-only",
        include_str!("../templates/licence/agpl-3.0-only.tera"),
    );
    m.insert(
        "lgpl-3.0-only",
        include_str!("../templates/licence/lgpl-3.0-only.tera"),
    );
    m.insert(
        "bsd-2-clause",
        include_str!("../templates/licence/bsd-2-clause.tera"),
    );
    m.insert(
        "bsd-3-clause",
        include_str!("../templates/licence/bsd-3-clause.tera"),
    );
    m.insert("mpl-2.0", include_str!("../templates/licence/mpl-2.0.tera"));
    m.insert(
        "unlicense",
        include_str!("../templates/licence/unlicense.tera"),
    );
    m.insert("cc0-1.0", include_str!("../templates/licence/cc0-1.0.tera"));
    m.insert("isc", include_str!("../templates/licence/isc.tera"));
    m.insert("wtfpl", include_str!("../templates/licence/wtfpl.tera"));
    m
});

/// Retrieve the built-in template text for a given SPDX identifier (case-insensitive).
pub fn get(spdx_lower: &str) -> Option<&'static str> {
    TEMPLATES.get(spdx_lower).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_returns_mit_template() {
        let template = get("mit").unwrap();
        assert!(template.contains("MIT"));
    }

    #[test]
    fn get_returns_apache_template() {
        let template = get("apache-2.0").unwrap();
        assert!(template.contains("Apache"));
    }

    #[test]
    fn get_returns_none_for_unknown() {
        assert!(get("nonexistent").is_none());
    }

    #[test]
    fn get_is_case_sensitive() {
        // Built-in templates are stored lowercase
        assert!(get("MIT").is_none()); // uppercase not in map
        assert!(get("mit").is_some()); // lowercase works
    }

    #[test]
    fn all_expected_templates_exist() {
        let expected = [
            "mit",
            "apache-2.0",
            "gpl-3.0-only",
            "gpl-2.0-only",
            "agpl-3.0-only",
            "lgpl-3.0-only",
            "bsd-2-clause",
            "bsd-3-clause",
            "mpl-2.0",
            "unlicense",
            "cc0-1.0",
            "isc",
            "wtfpl",
        ];
        for id in &expected {
            assert!(get(id).is_some(), "Missing template for: {}", id);
        }
    }
}
