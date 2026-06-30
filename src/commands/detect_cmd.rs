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
