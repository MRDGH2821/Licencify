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
            "licencify-config-{}-{}",
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
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, contents).unwrap();
    }

    fn run(&self, dir: &str, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_licencify"))
            .args(args)
            .current_dir(self.root.join(dir))
            .env("XDG_CONFIG_HOME", self.parent.join("xdg"))
            .env_remove("PRJ_ROOT")
            .env_remove("PRJ_CONFIG_HOME")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .output()
            .unwrap()
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

#[test]
fn config_commands_read_write_typed_fields_and_resolve_targets() {
    let project = Project::new();
    project.write(
        ".config/licencify/config.toml",
        "[default]\nadditional-licences = [\"Apache-2.0\"]\n",
    );

    let get = project.run(".", &["config", "get", "additional-licences"]);
    assert!(get.status.success(), "{}", text(&get.stderr));
    assert_eq!(text(&get.stdout).trim(), "[\"Apache-2.0\"]");

    let list = project.run(".", &["config", "list"]);
    assert!(list.status.success(), "{}", text(&list.stderr));
    assert!(text(&list.stdout).contains("Project config: ✓"));
    assert!(text(&list.stdout).contains("Global config: ✗"));
    assert!(text(&list.stdout).contains("Local config: ✗"));
    assert!(text(&list.stdout).contains("additional-licences = [\"Apache-2.0\"]"));

    let output = project.run(".", &["config", "set", "update-manifest=false"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let shared = fs::read_to_string(project.root.join(".config/licencify/config.toml")).unwrap();
    assert!(shared.contains("update-manifest = false"), "{shared}");
    let output = project.run(".", &["config", "get", "update-manifest"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    assert_eq!(text(&output.stdout).trim(), "false");

    let output = project.run(".", &["config", "set", "additional-licences=[\"MIT\"]"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let shared = fs::read_to_string(project.root.join(".config/licencify/config.toml")).unwrap();
    assert!(
        shared.contains("additional-licences = [\"MIT\"]"),
        "{shared}"
    );
    let output = project.run(".", &["config", "set", "additional-licences=[]"]);
    assert!(output.status.success(), "{}", text(&output.stderr));
    let shared = fs::read_to_string(project.root.join(".config/licencify/config.toml")).unwrap();
    assert!(shared.contains("additional-licences = []"), "{shared}");

    for basename in ["licence", "license", "copying"] {
        let output = project.run(".", &["config", "set", &format!("basename=\"{basename}\"")]);
        assert!(output.status.success(), "{}", text(&output.stderr));
        let shared =
            fs::read_to_string(project.root.join(".config/licencify/config.toml")).unwrap();
        assert!(
            shared.contains(&format!("basename = \"{basename}\"")),
            "{shared}"
        );
        let output = project.run(".", &["config", "get", "basename"]);
        assert!(output.status.success(), "{}", text(&output.stderr));
        assert_eq!(text(&output.stdout).trim(), basename);
    }
    let before = fs::read_to_string(project.root.join(".config/licencify/config.toml")).unwrap();
    let unknown = project.run(".", &["config", "set", "update_readme=true"]);
    assert!(unknown.status.success(), "{}", text(&unknown.stderr));
    assert!(text(&unknown.stderr).contains("unknown config key `update_readme`"));
    let unknown = project.run(".", &["config", "set", "unused=true"]);
    assert!(unknown.status.success(), "{}", text(&unknown.stderr));
    assert!(text(&unknown.stderr).contains("unknown config key `unused`"));
    assert_eq!(
        fs::read_to_string(project.root.join(".config/licencify/config.toml")).unwrap(),
        before
    );

    let invalid = project.run(".", &["config", "set", "basename=\"nonsense\""]);
    assert!(!invalid.status.success());
    assert!(text(&invalid.stderr).contains("Invalid config value"));
    assert_eq!(
        fs::read_to_string(project.root.join(".config/licencify/config.toml")).unwrap(),
        before
    );

    fs::create_dir_all(project.root.join("docs/api")).unwrap();
    let subdir = project.run("docs/api", &["config", "set", "company=\"Shared child\""]);
    assert!(subdir.status.success(), "{}", text(&subdir.stderr));
    let shared = fs::read_to_string(project.root.join(".config/licencify/config.toml")).unwrap();
    assert!(shared.contains("[[subdirs]]"), "{shared}");
    assert!(shared.contains("path = \"docs/api\""), "{shared}");
    assert!(shared.contains("company = \"Shared child\""), "{shared}");

    project.write(
        ".config/licencify/config.local.toml",
        "[[subdirs]]\npath = \"docs/api\"\nauthor = \"Deep author\"\n\n[[subdirs]]\npath = \"docs\"\nauthor = \"Parent author\"\n",
    );
    let masked = project.run("docs/api", &["config", "set", "author=\"Updated local\""]);
    assert!(masked.status.success(), "{}", text(&masked.stderr));
    assert!(text(&masked.stderr).contains("updated the masking local override"));
    let local =
        fs::read_to_string(project.root.join(".config/licencify/config.local.toml")).unwrap();
    assert!(
        local.contains("path = \"docs/api\"\nauthor = \"Updated local\""),
        "{local}"
    );
    assert!(
        local.contains("path = \"docs\"\nauthor = \"Parent author\""),
        "{local}"
    );
}

#[test]
fn licencify_has_no_schema_command() {
    let project = Project::new();
    let help = project.run(".", &["--help"]);
    assert!(help.status.success(), "{}", text(&help.stderr));
    assert!(
        !text(&help.stdout).contains("schema"),
        "licencify --help should not offer schema command"
    );

    let schema = project.run(".", &["schema"]);
    assert!(!schema.status.success(), "licencify schema command should fail");

    let config_schema = project.run(".", &["config", "schema"]);
    assert!(
        !config_schema.status.success(),
        "licencify config schema command should fail"
    );
}
