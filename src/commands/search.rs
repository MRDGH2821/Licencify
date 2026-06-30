use crate::{
    process::{RealRunner, Runner},
    provider::LicenseProvider,
};

pub fn cmd_search(query: &str, osi_only: bool, fsf_only: bool) -> anyhow::Result<()> {
    let runner = RealRunner;
    let prov = LicenseProvider::load()?;
    let results = prov.search(query);

    let mut results: Vec<_> = results
        .into_iter()
        .filter(|l| {
            if osi_only && !l.is_osi_approved {
                return false;
            }
            if fsf_only && !l.is_fsf_libre {
                return false;
            }
            true
        })
        .collect();
    results.sort_by(|a, b| a.license_id.cmp(&b.license_id));

    if results.is_empty() {
        eprintln!("No licenses found matching '{}'", query);
        runner.exit(1);
    }

    for license in &results {
        let mut flags = Vec::new();
        if license.is_osi_approved {
            flags.push("OSI");
        }
        if license.is_fsf_libre {
            flags.push("FSF");
        }
        if license.is_deprecated_license_id {
            flags.push("DEPRECATED");
        }
        let flag_str = if flags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", flags.join(", "))
        };

        let padded = format!("{:<40}", license.license_id);
        println!("{} {}{}", padded, license.name, flag_str);
    }

    println!("\n{} licenses found", results.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_search_finds_mit() {
        let result = cmd_search("MIT", false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn cmd_search_finds_gpl_via_partial() {
        let result = cmd_search("GPL", false, false);
        assert!(result.is_ok());
    }

    #[test]
    fn cmd_search_respects_osi_filter() {
        let result = cmd_search("MIT", true, false);
        assert!(result.is_ok());
    }
}
