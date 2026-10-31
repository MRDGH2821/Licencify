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

    fn config_path(&self) -> PathBuf {
        self.root.join(".config/licencify/config.toml")
    }
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
