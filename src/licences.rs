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

/// Return all supported SPDX identifiers for which built-in templates exist.
pub fn supported_ids() -> Vec<&'static str> {
    TEMPLATES.keys().copied().collect()
}
