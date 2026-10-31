use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

struct Project {
    parent: PathBuf,
    root: PathBuf,
}

impl Project {
    fn new() -> Self {
        let parent = std::env::temp_dir().join(format!(
            "licencify-apply-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = parent.join("project");
        fs::create_dir_all(root.join(".config/licencify")).unwrap();
        fs::create_dir_all(parent.join("xdg")).unwrap();
        fs::write(parent.join("gitconfig"), "").unwrap();
        let status = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .env("GIT_CONFIG_GLOBAL", parent.join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .status()
            .unwrap();
        assert!(status.success());
        let project = Self { parent, root };
        project.git_config("user.name", "Git Author");
        project.git_config("user.email", "git@example.com");
        project
    }

    fn git_config(&self, key: &str, value: &str) {
        let status = Command::new("git")
            .args(["config", key, value])
            .current_dir(&self.root)
            .env("GIT_CONFIG_GLOBAL", self.parent.join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .unwrap();
        assert!(status.success());
    }

    fn write(&self, rel: &str, contents: &str) {
        let path = self.root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn apply(&self, locale: &str) -> Output {
        self.apply_with_args(locale, &[])
    }

    fn apply_with_args(&self, locale: &str, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_licencify"))
            .arg("apply")
            .args(args)
            .current_dir(&self.root)
            .env("XDG_CONFIG_HOME", self.parent.join("xdg"))
            .env("GIT_CONFIG_GLOBAL", self.parent.join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("LANG", locale)
            .env_remove("LC_ALL")
            .env_remove("LC_MESSAGES")
            .env_remove("LANGUAGE")
            .env_remove("PRJ_ROOT")
            .env_remove("PRJ_CONFIG_HOME")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .output()
            .unwrap()
    }

    #[cfg(unix)]
    fn apply_interactive(&self, locale: &str, input: &str) -> (std::process::ExitStatus, String) {
        use std::io::{Read, Write};
        let (mut master, slave) = open_pty();
        let slave_out = slave.try_clone().unwrap();
        let slave_err = slave.try_clone().unwrap();

        let mut child = Command::new(env!("CARGO_BIN_EXE_licencify"))
            .arg("apply")
            .current_dir(&self.root)
            .env("XDG_CONFIG_HOME", self.parent.join("xdg"))
            .env("GIT_CONFIG_GLOBAL", self.parent.join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("LANG", locale)
            .env_remove("LC_ALL")
            .env_remove("LC_MESSAGES")
            .env_remove("LANGUAGE")
            .env_remove("PRJ_ROOT")
            .env_remove("PRJ_CONFIG_HOME")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .stdin(slave)
            .stdout(slave_out)
            .stderr(slave_err)
            .spawn()
            .unwrap();

        let _ = master.write_all(input.as_bytes());

        let mut output = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            match master.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
        let status = child.wait().unwrap();
        (status, String::from_utf8_lossy(&output).into_owned())
    }

    fn config_path(&self) -> PathBuf {
        self.root.join(".config/licencify/config.toml")
    }
}

#[cfg(unix)]
fn open_pty() -> (fs::File, fs::File) {
    use std::os::fd::FromRawFd;
    unsafe extern "C" {
        fn openpty(
            master: *mut std::ffi::c_int,
            slave: *mut std::ffi::c_int,
            name: *mut std::ffi::c_char,
            term: *const std::ffi::c_void,
            win: *const std::ffi::c_void,
        ) -> std::ffi::c_int;
    }
    let mut master = 0;
    let mut slave = 0;
    let res = unsafe {
        openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
        )
    };
    assert_eq!(res, 0, "openpty failed");
    unsafe { (fs::File::from_raw_fd(master), fs::File::from_raw_fd(slave)) }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.parent);
    }
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn assert_success(output: &Output) {
    assert!(output.status.success(), "{}", text(&output.stderr));
}

#[test]
fn apply_writes_configured_licence_without_prompt_or_config_changes() {
    let project = Project::new();
    let config = "[default]\nlicence = \"MIT\"\nauthor = \"Config Author\"\nyear = \"2024\"\n";
    project.write(".config/licencify/config.toml", config);

    let output = project.apply("en_US.UTF-8");

    assert_success(&output);
    assert!(text(&output.stdout).contains("LICENSE.txt"));
    assert!(!text(&output.stdout).contains("?"));
    assert!(
        fs::read_to_string(project.root.join("LICENSE.txt"))
            .unwrap()
            .contains("Copyright (c) 2024 Config Author")
    );
    assert_eq!(fs::read_to_string(project.config_path()).unwrap(), config);
}

#[test]
fn apply_requires_an_effective_licence() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nauthor = \"Author\"\n",
    );

    let output = project.apply("en_US.UTF-8");

    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("licence"));
    assert!(!project.root.join("LICENSE.txt").exists());
}

#[test]
fn apply_uses_configured_basename_and_format() {
    for (basename, filename) in [
        ("licence", "LICENCE.html"),
        ("license", "LICENSE.html"),
        ("copying", "COPYING.html"),
    ] {
        let project = Project::new();
        project.write(
            ".config/licencify/config.toml",
            &format!(
                "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\nbasename = \"{basename}\"\nformat = \"html\"\n"
            ),
        );

        let output = project.apply("en_US.UTF-8");

        assert_success(&output);
        assert!(text(&output.stdout).contains(filename));
        assert!(project.root.join(filename).exists());
    }
}

#[test]
fn apply_uses_detected_locale_when_basename_is_unset() {
    for (locale, filename) in [
        ("en_GB.UTF-8", "LICENCE.txt"),
        ("en_US.UTF-8", "LICENSE.txt"),
    ] {
        let project = Project::new();
        project.write(
            ".config/licencify/config.toml",
            "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\n",
        );

        let output = project.apply(locale);

        assert_success(&output);
        assert!(text(&output.stdout).contains(filename));
        assert!(project.root.join(filename).exists());
    }
}

#[test]
fn apply_creates_missing_additional_and_preserves_existing_additional_files() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\nadditional-licences = [\"Apache-2.0\", \"BSD-3-Clause\"]\n",
    );
    project.write("LICENSE-BSD-3-Clause.md", "keep this existing text\n");

    let output = project.apply("en_US.UTF-8");

    assert_success(&output);
    assert!(project.root.join("LICENSE-Apache-2.0.txt").exists());
    assert_eq!(
        fs::read_to_string(project.root.join("LICENSE-BSD-3-Clause.md")).unwrap(),
        "keep this existing text\n"
    );
    assert!(!project.root.join("LICENSE-BSD-3-Clause.txt").exists());
}

#[test]
fn apply_reports_git_fallbacks_and_fails_without_git_name() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"proprietary\"\nyear = \"2024\"\n",
    );

    let output = project.apply("en_US.UTF-8");

    assert_success(&output);
    let stdout = text(&output.stdout);
    assert!(
        stdout.contains("author") && stdout.contains("git config"),
        "{stdout}"
    );
    assert!(
        stdout.contains("email") && stdout.contains("git config"),
        "{stdout}"
    );
    let licence = fs::read_to_string(project.root.join("LICENSE.txt")).unwrap();
    assert!(licence.contains("Git Author"));
    assert!(licence.contains("git@example.com"));

    let missing_name = Project::new();
    missing_name.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\n",
    );
    let status = Command::new("git")
        .args(["config", "--unset", "user.name"])
        .current_dir(&missing_name.root)
        .env("GIT_CONFIG_GLOBAL", missing_name.parent.join("gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .unwrap();
    assert!(status.success());

    let output = missing_name.apply("en_US.UTF-8");

    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("No author specified"));
    assert!(!missing_name.root.join("LICENSE.txt").exists());
}

#[test]
fn apply_leaves_email_empty_when_git_has_no_email() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"proprietary\"\nyear = \"2024\"\n",
    );
    let status = Command::new("git")
        .args(["config", "--unset", "user.email"])
        .current_dir(&project.root)
        .env("GIT_CONFIG_GLOBAL", project.parent.join("gitconfig"))
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .unwrap();
    assert!(status.success());

    let output = project.apply("en_US.UTF-8");

    assert_success(&output);
    let licence = fs::read_to_string(project.root.join("LICENSE.txt")).unwrap();
    assert!(licence.contains("Git Author"));
    assert!(!licence.contains("git@example.com"));
    assert!(licence.contains("Git Author <>"));
}

#[test]
fn apply_fails_and_leaves_files_unchanged_when_multiple_primaries_exist() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\n",
    );
    project.write("LICENSE", "first primary\n");
    project.write("LICENCE.txt", "second primary\n");

    let output = project.apply("en_US.UTF-8");

    assert!(!output.status.success());
    assert!(
        text(&output.stderr).contains("Multiple primary licence files"),
        "{}",
        text(&output.stderr)
    );
    assert_eq!(
        fs::read_to_string(project.root.join("LICENSE")).unwrap(),
        "first primary\n"
    );
    assert_eq!(
        fs::read_to_string(project.root.join("LICENCE.txt")).unwrap(),
        "second primary\n"
    );
}

#[test]
fn apply_non_interactive_fails_and_leaves_existing_primary_when_confirmation_needed() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\n",
    );
    project.write("LICENSE.txt", "Old primary content\n");

    let output = project.apply("en_US.UTF-8");

    assert!(!output.status.success());
    let stderr = text(&output.stderr);
    assert!(
        stderr.contains("--yes") && stderr.contains("LICENSE.txt"),
        "{stderr}"
    );
    assert_eq!(
        fs::read_to_string(project.root.join("LICENSE.txt")).unwrap(),
        "Old primary content\n"
    );
}

#[test]
fn apply_with_yes_skips_question_and_replaces_existing_primary() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"New Author\"\nyear = \"2025\"\n",
    );
    project.write("LICENSE.txt", "Old primary content\n");

    let output = project.apply_with_args("en_US.UTF-8", &["--yes"]);

    assert_success(&output);
    assert!(!text(&output.stdout).contains("?"));
    let new_content = fs::read_to_string(project.root.join("LICENSE.txt")).unwrap();
    assert!(new_content.contains("Copyright (c) 2025 New Author"));
    assert!(!new_content.contains("Old primary content"));
}

#[test]
#[cfg(unix)]
fn apply_interactive_replaces_existing_primary_after_confirmation() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Interactive Author\"\nyear = \"2026\"\n",
    );
    project.write("LICENSE.txt", "Old primary content\n");

    let (status, output) = project.apply_interactive("en_US.UTF-8", "y\n");

    assert!(status.success(), "{output}");
    assert!(
        output.contains("About to replace the primary licence") && output.contains("?"),
        "{output}"
    );
    let new_content = fs::read_to_string(project.root.join("LICENSE.txt")).unwrap();
    assert!(new_content.contains("Copyright (c) 2026 Interactive Author"));
    assert!(!new_content.contains("Old primary content"));
}

#[test]
#[cfg(unix)]
fn apply_interactive_aborts_and_leaves_existing_primary_when_declined() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Interactive Author\"\nyear = \"2026\"\n",
    );
    project.write("LICENSE.txt", "Old primary content\n");

    let (status, output) = project.apply_interactive("en_US.UTF-8", "n\n");

    assert!(status.success(), "{output}");
    assert!(output.contains("Aborted."), "{output}");
    let content = fs::read_to_string(project.root.join("LICENSE.txt")).unwrap();
    assert_eq!(content, "Old primary content\n");
}

#[test]
fn apply_removes_old_primary_variant_only_after_replacement_is_ready() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\nbasename = \"license\"\nformat = \"html\"\n",
    );
    project.write("LICENCE", "Old LICENCE file\n");

    let output = project.apply_with_args("en_US.UTF-8", &["--yes"]);

    assert_success(&output);
    assert!(project.root.join("LICENSE.html").exists());
    assert!(!project.root.join("LICENCE").exists());
}

#[test]
#[cfg(unix)]
fn apply_failed_write_leaves_old_primary_in_place() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\n",
    );
    project.write("LICENSE.txt", "Old untouched primary\n");

    let mut perms = fs::metadata(&project.root).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&project.root, perms).unwrap();

    let output = project.apply_with_args("en_US.UTF-8", &["--yes"]);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&project.root, fs::Permissions::from_mode(0o755)).unwrap();
    }

    assert!(!output.status.success());
    let content = fs::read_to_string(project.root.join("LICENSE.txt")).unwrap();
    assert_eq!(content, "Old untouched primary\n");
}

#[test]
fn apply_keeps_readme_unchanged_by_default() {
    let project = Project::new();
    let readme =
        "# Project\n\nLicensed under GPL-3.0-only. Handwritten [licence](LICENCE.txt) reference.\n";
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\n",
    );
    project.write("README.md", readme);

    let output = project.apply("en_US.UTF-8");

    assert_success(&output);
    assert!(text(&output.stdout).contains("Applied"));
    assert!(!text(&output.stderr).contains("conflict"));
    assert!(!text(&output.stderr).contains("Warning"));
    assert_eq!(
        fs::read_to_string(project.root.join("README.md")).unwrap(),
        readme
    );
    assert!(project.root.join("LICENSE.txt").exists());
}

#[test]
fn apply_updates_readme_only_when_enabled_and_warns_about_old_references() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\n",
    );
    project.write(
        "README.md",
        "# Project\n\nHandwritten note stays.\n\nSee [the old file](LICENCE.txt).\n",
    );

    let output = project.apply_with_args("en_US.UTF-8", &["--update-readme"]);

    assert_success(&output);
    let stdout = text(&output.stdout);
    let stderr = text(&output.stderr);
    let readme = fs::read_to_string(project.root.join("README.md")).unwrap();
    assert!(stdout.contains("Applied"));
    assert!(stdout.contains("Updated README.md"));
    assert!(stderr.contains("LICENCE.txt"), "{stderr}");
    assert!(stderr.contains("LICENSE.txt"), "{stderr}");
    assert!(readme.contains("Handwritten note stays."));
    assert!(readme.contains("](LICENSE.txt)"));
    assert!(project.root.join("LICENSE.txt").exists());
}

#[test]
fn apply_uses_enabled_readme_configuration_when_no_flag_is_given() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\nupdate-readme = true\n",
    );
    project.write("README.md", "# Project\n\nHandwritten note stays.\n");

    let output = project.apply("en_US.UTF-8");

    assert_success(&output);
    assert!(text(&output.stdout).contains("Updated README.md"));
    let readme = fs::read_to_string(project.root.join("README.md")).unwrap();
    assert!(readme.contains("Handwritten note stays."));
    assert!(readme.contains("](LICENSE.txt)"));
    assert!(project.root.join("LICENSE.txt").exists());
}

#[test]
fn apply_no_update_readme_overrides_enabled_configuration() {
    let project = Project::new();
    let readme = "# Project\n\nHandwritten note stays.\n";
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\nupdate-readme = true\n",
    );
    project.write("README.md", readme);

    let output = project.apply_with_args("en_US.UTF-8", &["--no-update-readme"]);

    assert_success(&output);
    assert!(!text(&output.stdout).contains("Updated README.md"));
    assert_eq!(
        fs::read_to_string(project.root.join("README.md")).unwrap(),
        readme
    );
    assert!(project.root.join("LICENSE.txt").exists());
}

#[test]
fn apply_rejects_conflicting_readme_flags_without_writing_files() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\n",
    );
    project.write("README.md", "# Project\n");

    let output = project.apply_with_args("en_US.UTF-8", &["--update-readme", "--no-update-readme"]);

    assert!(!output.status.success());
    assert!(text(&output.stderr).contains("cannot be used with"));
    assert_eq!(
        fs::read_to_string(project.root.join("README.md")).unwrap(),
        "# Project\n"
    );
    assert!(!project.root.join("LICENSE.txt").exists());
}

#[test]
fn apply_with_readme_updates_enabled_succeeds_when_readme_is_missing() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\n",
    );

    let output = project.apply_with_args("en_US.UTF-8", &["--update-readme"]);

    assert_success(&output);
    assert!(text(&output.stdout).contains("Applied"));
    assert!(!text(&output.stdout).contains("Updated README"));
    assert!(!text(&output.stderr).contains("Warning"));
    assert!(!project.root.join("README.md").exists());
    assert!(project.root.join("LICENSE.txt").exists());
}

#[test]
fn apply_with_readme_updates_enabled_keeps_equivalent_readme_section() {
    let project = Project::new();
    let readme = "# Project\n\n[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE.txt)\n\n## License\n\nThis project is licensed under the [MIT](LICENSE.txt) licence.\n";
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\n",
    );
    project.write("README.md", readme);

    let output = project.apply_with_args("en_US.UTF-8", &["--update-readme"]);

    assert_success(&output);
    assert!(text(&output.stdout).contains("Applied"));
    assert!(!text(&output.stdout).contains("Updated README.md"));
    assert_eq!(
        fs::read_to_string(project.root.join("README.md")).unwrap(),
        readme
    );
    assert!(project.root.join("LICENSE.txt").exists());
}

#[test]
fn apply_with_readme_updates_enabled_leaves_handwritten_equivalent_section_unchanged() {
    let project = Project::new();
    let readme = "# Project\n\n## License\n\nThis project is licensed under the MIT licence.\n";
    project.write(
        ".config/licencify/config.toml",
        "[default]\nlicence = \"MIT\"\nauthor = \"Author\"\n",
    );
    project.write("README.md", readme);

    let output = project.apply_with_args("en_US.UTF-8", &["--update-readme"]);

    assert_success(&output);
    assert!(text(&output.stdout).contains("Applied"));
    assert!(!text(&output.stdout).contains("Updated README.md"));
    assert_eq!(
        fs::read_to_string(project.root.join("README.md")).unwrap(),
        readme
    );
    assert!(project.root.join("LICENSE.txt").exists());
}
