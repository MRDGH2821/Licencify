use anyhow::Context;
use licencify::config::Config;
use std::fs;
use std::path::Path;
use std::process::Command;

fn main() -> anyhow::Result<()> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target_path = manifest_dir.join("schema/config.json");

    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let json = Config::schema_json()?;
    fs::write(&target_path, json)
        .with_context(|| format!("Failed to write schema to {}", target_path.display()))?;

    let _ = Command::new("yq")
        .args(["-iP", "sort_keys(..)", &target_path.to_string_lossy()])
        .status();
    let _ = Command::new("oxfmt").arg(&target_path).status();

    println!("Wrote schema to {}", target_path.display());
    Ok(())
}
