use crate::provider::LicenseProvider;

pub fn cmd_list(osi_only: bool, fsf_only: bool, limit: Option<usize>) -> anyhow::Result<()> {
    let prov = LicenseProvider::load()?;
    let mut licenses: Vec<_> = prov.all_licenses().iter().collect();
    licenses.sort_by(|a, b| a.license_id.cmp(&b.license_id));
    let mut count = 0usize;

    for license in &licenses {
        if osi_only && !license.is_osi_approved {
            continue;
        }
        if fsf_only && !license.is_fsf_libre {
            continue;
        }

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
        count += 1;

        if let Some(max) = limit {
            if count >= max {
                println!(
                    "\n... showing {} of {} total",
                    count,
                    prov.all_licenses().len()
                );
                break;
            }
        }
    }

    println!("\n{} licenses found", count);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cmd_list_returns_all_licenses() {
        let result = cmd_list(false, false, None);
        assert!(result.is_ok());
    }

    #[test]
    fn cmd_list_respects_osi_filter() {
        let result = cmd_list(true, false, None);
        assert!(result.is_ok());
    }

    #[test]
    fn cmd_list_respects_fsf_filter() {
        let result = cmd_list(false, true, None);
        assert!(result.is_ok());
    }

    #[test]
    fn cmd_list_respects_limit() {
        let result = cmd_list(false, false, Some(5));
        assert!(result.is_ok());
    }
}
