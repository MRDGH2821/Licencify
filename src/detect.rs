/// Pure license detection from file text content.
/// Returns the SPDX license ID if detected, or `None`.
pub fn detect_license(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();

    if lower.contains("mit license")
        || lower.contains("permission is hereby granted, free of charge")
    {
        return Some("MIT");
    }
    if lower.contains("mozilla public license") && lower.contains("version 2.0") {
        return Some("MPL-2.0");
    }
    if lower.contains("apache license") || lower.contains("version 2.0") {
        return Some("Apache-2.0");
    }
    if lower.contains("gnu general public license") && lower.contains("version 3") {
        return Some("GPL-3.0-only");
    }
    if lower.contains("gnu general public license") && lower.contains("version 2") {
        return Some("GPL-2.0-only");
    }
    if lower.contains("gnu lesser general public license") {
        return Some("LGPL-3.0-only");
    }
    if lower.contains("bsd") && lower.contains("redistribution and use") {
        if lower.contains("neither the name of") {
            return Some("BSD-3-Clause");
        } else {
            return Some("BSD-2-Clause");
        }
    }
    if lower.contains("isc license")
        || lower.contains("permission to use, copy, modify, and/or distribute this software")
    {
        return Some("ISC");
    }
    if lower.contains("unlicense")
        || lower.contains("this is free and unencumbered software released into the public domain")
    {
        return Some("Unlicense");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_mit() {
        let text = "MIT License\n\nCopyright (c) 2024 Test\n\n\
                     Permission is hereby granted, free of charge...";
        assert_eq!(detect_license(text), Some("MIT"));
    }

    #[test]
    fn detect_apache() {
        let text = "Apache License\nVersion 2.0\n\nLicensed under the Apache License...";
        assert_eq!(detect_license(text), Some("Apache-2.0"));
    }

    #[test]
    fn detect_gpl3() {
        let text = "GNU GENERAL PUBLIC LICENSE\nVersion 3";
        assert_eq!(detect_license(text), Some("GPL-3.0-only"));
    }

    #[test]
    fn detect_gpl2() {
        let text = "GNU GENERAL PUBLIC LICENSE\nVersion 2";
        assert_eq!(detect_license(text), Some("GPL-2.0-only"));
    }

    #[test]
    fn detect_lgpl() {
        let text = "GNU LESSER GENERAL PUBLIC LICENSE";
        assert_eq!(detect_license(text), Some("LGPL-3.0-only"));
    }

    #[test]
    fn detect_mpl() {
        let text = "Mozilla Public License\nVersion 2.0";
        assert_eq!(detect_license(text), Some("MPL-2.0"));
    }

    #[test]
    fn detect_bsd3() {
        let text = "BSD\nRedistribution and use\nNeither the name of";
        assert_eq!(detect_license(text), Some("BSD-3-Clause"));
    }

    #[test]
    fn detect_bsd2() {
        let text = "BSD\nRedistribution and use";
        assert_eq!(detect_license(text), Some("BSD-2-Clause"));
    }

    #[test]
    fn detect_isc() {
        let text = "ISC License\n\nPermission to use, copy, modify, and/or distribute";
        assert_eq!(detect_license(text), Some("ISC"));
    }

    #[test]
    fn detect_unlicense() {
        let text = "This is free and unencumbered software released into the public domain";
        assert_eq!(detect_license(text), Some("Unlicense"));
    }

    #[test]
    fn detect_unknown() {
        assert_eq!(
            detect_license("Some random text with no license keywords"),
            None
        );
    }
}
