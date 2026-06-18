use crate::detect;

pub fn cmd_detect() -> anyhow::Result<()> {
    let candidates = [
        "LICENSE",
        "LICENSE.txt",
        "LICENSE.md",
        "COPYING",
        "COPYING.txt",
    ];

    for name in &candidates {
        if std::path::Path::new(name).exists() {
            let content = std::fs::read_to_string(name)?;
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

    eprintln!("No license file found in current directory");
    eprintln!("Hint: use 'licencify add <SPDX-ID>' to add one");
    std::process::exit(1);
}
