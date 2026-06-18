mod agpl3;
mod apache;
mod bsd2;
mod bsd3;
mod cc01;
mod gpl2;
mod gpl3;
mod isc;
mod lgpl3;
mod mit;
mod mpl2;
mod unlicense;
mod wtfpl;

use std::collections::HashMap;
use std::sync::LazyLock;

static TEMPLATES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m: HashMap<&'static str, &'static str> = HashMap::new();
    m.insert("mit", mit::TEXT);
    m.insert("apache-2.0", apache::TEXT);
    m.insert("gpl-3.0-only", gpl3::TEXT);
    m.insert("gpl-2.0-only", gpl2::TEXT);
    m.insert("agpl-3.0-only", agpl3::TEXT);
    m.insert("lgpl-3.0-only", lgpl3::TEXT);
    m.insert("bsd-2-clause", bsd2::TEXT);
    m.insert("bsd-3-clause", bsd3::TEXT);
    m.insert("mpl-2.0", mpl2::TEXT);
    m.insert("unlicense", unlicense::TEXT);
    m.insert("cc0-1.0", cc01::TEXT);
    m.insert("isc", isc::TEXT);
    m.insert("wtfpl", wtfpl::TEXT);
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
