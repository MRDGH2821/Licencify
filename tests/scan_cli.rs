use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::Command;

struct Project {
    parent: PathBuf,
    root: PathBuf,
}

impl Project {
    fn new() -> Self {
        let parent = std::env::temp_dir().join(format!(
            "licencify-scan-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = parent.join("project");
        fs::create_dir_all(root.join(".config/licencify")).unwrap();
        fs::create_dir_all(parent.join("xdg")).unwrap();
        let status = Command::new("git")
            .arg("init")
            .current_dir(&root)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .status()
            .unwrap();
        assert!(status.success());
        Self { parent, root }
    }

    fn write(&self, rel: &str, contents: &str) {
        let path = self.root.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }

    fn scan(&self, id: Option<&str>) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_licencify"));
        command
            .current_dir(&self.root)
            .env("XDG_CONFIG_HOME", self.parent.join("xdg"))
            .env_remove("PRJ_ROOT")
            .env_remove("PRJ_CONFIG_HOME")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE");
        if let Some(id) = id {
            command.arg("scan").arg(id);
        } else {
            command.arg("scan");
        }
        command.output().unwrap()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.parent);
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn tree(root: &Path) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let meta = fs::symlink_metadata(&path).unwrap();
            if meta.file_type().is_symlink() {
                out.push(format!("{rel} -> symlink"));
                continue;
            }
            if meta.is_dir() {
                walk(root, &path, out);
            } else {
                out.push(rel);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, root, &mut out);
    out.sort();
    out
}

#[test]
fn scan_reports_evidence_without_writing() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "\
[default]
licence = \"MIT\"
additional-licences = [\"Apache-2.0\"]

[scan]
exclude = [\"third_party\", \"vendor\"]

[[subdirs]]
path = \"vendor\"
licence = \"ISC\"
additional-licences = []

[[subdirs]]
path = \"missing/place\"
licence = \"MIT\"

[[subdirs]]
path = \"hole\"
",
    );
    project.write(
        ".config/licencify/config.local.toml",
        "[scan]\nexclude = [\"build\"]\n",
    );
    project.write(".gitignore", "deps/\n");
    project.write(
        "LICENSE",
        "MIT License\n\nPermission is hereby granted, free of charge\n",
    );
    project.write("LICENCE-Apache-2.0.txt", "Apache License\nVersion 2.0\n");
    project.write("COPYING.md", "This repository notes local conventions.\n");
    project.write(
        "README.md",
        "\
# Sample

License: MIT

```
License: GPL-3.0-only
```
",
    );
    project.write(
        "pkg/LICENSE",
        "MIT License\n\nPermission is hereby granted, free of charge\n",
    );
    project.write(
        "pkg/LICENCE-ISC.txt",
        "MIT License\n\nPermission is hereby granted, free of charge\n",
    );
    project.write(
        "vendor/LICENSE",
        "ISC License\n\nPermission to use, copy, modify, and/or distribute this software\n",
    );
    project.write("third_party/LICENSE", "MIT License\n");
    project.write("deps/LICENSE", "MIT License\n");
    project.write("build/LICENSE", "MIT License\n");
    project.write("hole/.keep", "");
    let outside = project.parent.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("LICENSE"), "OUTSIDE-LEAK-ID\nMIT License\n").unwrap();
    symlink(&outside, project.root.join("escape")).unwrap();

    let locked = project.root.join("locked/LICENCE");
    project.write("locked/LICENCE", "notes only\n");
    let mut perms = fs::metadata(&locked).unwrap().permissions();
    perms.set_mode(0o000);
    fs::set_permissions(&locked, perms).unwrap();
    let unreadable = fs::read(&locked).is_err();

    let license_before = fs::read(project.root.join("LICENSE")).unwrap();
    let before = tree(&project.root);
    let output = project.scan(None);
    let stdout = text(&output.stdout);
    let stderr = text(&output.stderr);

    assert_eq!(
        fs::read(project.root.join("LICENSE")).unwrap(),
        license_before
    );
    assert_eq!(tree(&project.root), before, "scan changed the tree");
    assert!(!stdout.contains("OUTSIDE-LEAK-ID"));
    assert!(
        stderr.contains("Warning: skipped symlink escape outside the project root"),
        "{stderr}"
    );
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(lines.contains(&"LICENSE: identified MIT"));
    assert!(lines.contains(&"LICENCE-Apache-2.0.txt: identified Apache-2.0"));
    assert!(lines.contains(&"COPYING.md: unknown"));
    assert!(lines.contains(&"README.md: claimed MIT"));
    assert!(!stdout.contains("GPL-3.0-only"));
    assert!(lines.contains(&"pkg/LICENSE: identified MIT"));
    assert!(lines.contains(&"pkg/LICENCE-ISC.txt: identified MIT"));
    assert!(lines.contains(&"vendor/LICENSE: identified ISC"));
    assert!(!stdout.contains("third_party"));
    assert!(!stdout.contains("deps/"));
    assert!(!stdout.contains("build/"));
    assert!(stderr.contains("Warning: unknown licence content in COPYING.md"));
    assert!(stderr.contains("Warning: missing configured directory missing/place"));
    assert!(stderr.contains("Warning: missing expected primary MIT in hole"));
    assert!(stderr.contains("Warning: missing expected additional Apache-2.0 in hole"));
    assert!(stderr.contains(
        "Warning: scan exclusion `vendor` overlaps configured subdir `vendor`; scanning that subdir anyway"
    ));
    assert!(stderr.contains("Error: conflict pkg/LICENCE-ISC.txt: identified MIT, claimed ISC"));
    if unreadable {
        assert!(stderr.contains("Error: unreadable locked/LICENCE"));
        assert!(!stdout.contains("locked/LICENCE:"));
    } else {
        assert!(stdout.contains("locked/LICENCE: unknown"));
    }
    assert!(!output.status.success());
}

#[test]
fn explicit_scan_id_overrides_only_the_primary() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "\
[default]
licence = \"MIT\"
additional-licences = [\"Apache-2.0\"]

[[subdirs]]
path = \"extras\"
",
    );
    fs::create_dir_all(project.root.join("extras")).unwrap();
    project.write(
        "LICENSE",
        "MIT License\n\nPermission is hereby granted, free of charge\n",
    );
    project.write("LICENCE-Apache-2.0.txt", "Apache License\nVersion 2.0\n");

    let plain = project.scan(None);
    let plain_out = text(&plain.stdout);
    let plain_err = text(&plain.stderr);
    assert!(plain.status.success(), "{plain_err}");
    assert!(plain_out.contains("LICENSE: identified MIT"));
    assert!(plain_out.contains("LICENCE-Apache-2.0.txt: identified Apache-2.0"));
    assert!(plain_err.contains("Warning: missing expected primary MIT in extras"));
    assert!(plain_err.contains("Warning: missing expected additional Apache-2.0 in extras"));
    assert!(!plain_err.contains("Error:"));

    let output = project.scan(Some("GPL-3.0-only"));
    let stdout = text(&output.stdout);
    let stderr = text(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(stdout.contains("LICENCE-Apache-2.0.txt: identified Apache-2.0"));
    assert!(stderr.contains("Error: conflict LICENSE: identified MIT is not an expected licence"));
    assert!(!stderr.contains("identified Apache-2.0 is not an expected licence"));
    assert!(stderr.contains("Warning: missing expected primary GPL-3.0-only in extras"));
    assert!(stderr.contains("Warning: missing expected additional Apache-2.0 in extras"));
}
