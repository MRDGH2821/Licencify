use std::collections::HashMap;
use std::sync::LazyLock;

/// A licence template with both plain text and HTML variants.
pub struct TemplateSet {
    pub txt: &'static str,
    pub html: &'static str,
}

static TEMPLATES: LazyLock<HashMap<&'static str, TemplateSet>> = LazyLock::new(|| {
    let mut m: HashMap<&'static str, TemplateSet> = HashMap::new();
    m.insert(
        "mit",
        TemplateSet {
            txt: include_str!("../templates/licence/mit.tera"),
            html: include_str!("../templates/licence/mit.html.tera"),
        },
    );
    m.insert(
        "apache-2.0",
        TemplateSet {
            txt: include_str!("../templates/licence/apache-2.0.tera"),
            html: include_str!("../templates/licence/apache-2.0.html.tera"),
        },
    );
    m.insert(
        "gpl-3.0-only",
        TemplateSet {
            txt: include_str!("../templates/licence/gpl-3.0-only.tera"),
            html: include_str!("../templates/licence/gpl-3.0-only.html.tera"),
        },
    );
    m.insert(
        "gpl-2.0-only",
        TemplateSet {
            txt: include_str!("../templates/licence/gpl-2.0-only.tera"),
            html: include_str!("../templates/licence/gpl-2.0-only.html.tera"),
        },
    );
    m.insert(
        "agpl-3.0-only",
        TemplateSet {
            txt: include_str!("../templates/licence/agpl-3.0-only.tera"),
            html: include_str!("../templates/licence/agpl-3.0-only.html.tera"),
        },
    );
    m.insert(
        "lgpl-3.0-only",
        TemplateSet {
            txt: include_str!("../templates/licence/lgpl-3.0-only.tera"),
            html: include_str!("../templates/licence/lgpl-3.0-only.html.tera"),
        },
    );
    m.insert(
        "bsd-2-clause",
        TemplateSet {
            txt: include_str!("../templates/licence/bsd-2-clause.tera"),
            html: include_str!("../templates/licence/bsd-2-clause.html.tera"),
        },
    );
    m.insert(
        "bsd-3-clause",
        TemplateSet {
            txt: include_str!("../templates/licence/bsd-3-clause.tera"),
            html: include_str!("../templates/licence/bsd-3-clause.html.tera"),
        },
    );
    m.insert(
        "mpl-2.0",
        TemplateSet {
            txt: include_str!("../templates/licence/mpl-2.0.tera"),
            html: include_str!("../templates/licence/mpl-2.0.html.tera"),
        },
    );
    m.insert(
        "unlicense",
        TemplateSet {
            txt: include_str!("../templates/licence/unlicense.tera"),
            html: include_str!("../templates/licence/unlicense.html.tera"),
        },
    );
    m.insert(
        "cc0-1.0",
        TemplateSet {
            txt: include_str!("../templates/licence/cc0-1.0.tera"),
            html: include_str!("../templates/licence/cc0-1.0.html.tera"),
        },
    );
    m.insert(
        "isc",
        TemplateSet {
            txt: include_str!("../templates/licence/isc.tera"),
            html: include_str!("../templates/licence/isc.html.tera"),
        },
    );
    m.insert(
        "wtfpl",
        TemplateSet {
            txt: include_str!("../templates/licence/wtfpl.tera"),
            html: include_str!("../templates/licence/wtfpl.html.tera"),
        },
    );
    m.insert(
        "proprietary",
        TemplateSet {
            txt: include_str!("../templates/licence/proprietary.tera"),
            html: include_str!("../templates/licence/proprietary.html.tera"),
        },
    );
    m
});

/// Retrieve the built-in template set for a given SPDX identifier (case-insensitive).
pub fn get(spdx_lower: &str) -> Option<&'static TemplateSet> {
    TEMPLATES.get(spdx_lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_returns_mit_templates() {
        let tmpl = get("mit").unwrap();
        assert!(tmpl.txt.contains("MIT"));
        assert!(tmpl.html.contains("MIT"));
        assert!(tmpl.html.contains("<!DOCTYPE html>"));
        assert!(tmpl.html.contains("<p>MIT License</p>"));
    }

    #[test]
    fn get_returns_apache_templates() {
        let tmpl = get("apache-2.0").unwrap();
        assert!(tmpl.txt.contains("Apache"));
        assert!(tmpl.html.contains("Apache"));
    }

    #[test]
    fn get_returns_none_for_unknown() {
        assert!(get("nonexistent").is_none());
    }

    #[test]
    fn get_is_case_sensitive() {
        assert!(get("MIT").is_none());
        assert!(get("mit").is_some());
    }

    #[test]
    fn get_returns_proprietary_templates() {
        let tmpl = get("proprietary").unwrap();
        assert!(tmpl.txt.contains("All Rights Reserved"));
        assert!(tmpl.txt.contains("Proprietary and confidential"));
        assert!(tmpl.html.contains("<!DOCTYPE html>"));
        assert!(tmpl.html.contains("<pre>"));
        assert!(tmpl.html.contains("All Rights Reserved"));
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
            "proprietary",
        ];
        for id in &expected {
            let tmpl = get(id).unwrap_or_else(|| panic!("Missing template for: {}", id));
            assert!(!tmpl.txt.is_empty(), "Empty txt template for: {}", id);
            assert!(
                tmpl.html.contains("<!DOCTYPE html>"),
                "HTML template for {} doesn't contain <!DOCTYPE html>",
                id
            );
            // SPDX templates use semantic HTML (<p>, <div>), proprietary uses <pre>
        }
    }
}
