use crate::provider::LicenseProvider;

pub fn cmd_search(query: &str, osi_only: bool, fsf_only: bool) -> anyhow::Result<()> {
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
        std::process::exit(1);
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
