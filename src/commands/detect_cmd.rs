use crate::{detect, fs::global_fs, licence_name::LicenceName};

pub fn cmd_detect() -> anyhow::Result<()> {
    let fs = global_fs();

    for name in LicenceName::candidates() {
        let path = std::path::Path::new(name);
        if fs.exists(path) {
            if let Some(content) = fs.read_to_string(path) {
                if let Some(spdx_id) = detect::detect_license(&content) {
                    println!("Detected: {} ({})", spdx_id, name);
                    return Ok(());
                } else {
                    println!("Found {} but could not determine license type", name);
                    println!("Hint: use 'licencify search' to find the right SPDX ID");
                    return Ok(());
                }
            }
        }
    }

    eprintln!("No license file found in current directory");
    eprintln!("Hint: use 'licencify add <SPDX-ID>' to add one");
    anyhow::bail!("No license file found in current directory");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{FsGuard, MemFs};
    use std::sync::Arc;

    #[test]
    fn cmd_detect_finds_mit_license() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(
            std::path::Path::new("LICENSE"),
            "MIT License\n\nPermission is hereby granted, free of charge...",
        )
        .unwrap();
        let result = cmd_detect();
        assert!(result.is_ok());
    }

    #[test]
    fn cmd_detect_returns_err_when_no_license() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        let result = cmd_detect();
        assert!(result.is_err());
    }

    #[test]
    fn cmd_detect_finds_unlicensed() {
        let _guard = FsGuard::new();
        let fs = Arc::new(MemFs::new()) as Arc<dyn crate::fs::Fs>;
        crate::fs::set_global_fs(fs.clone());
        fs.write(
            std::path::Path::new("LICENSE"),
            "All Rights Reserved\nProprietary and confidential",
        )
        .unwrap();
        let result = cmd_detect();
        assert!(result.is_ok());
    }
}
