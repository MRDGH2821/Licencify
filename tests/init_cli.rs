use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const EXPECTED_SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/MRDGH2821/Licencify/main/schema/config.json";

struct Project {
    parent: PathBuf,
    root: PathBuf,
    xdg: PathBuf,
}

impl Project {
    fn new() -> Self {
        let parent = std::env::temp_dir().join(format!(
            "licencify-init-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let root = parent.join("project");
        let xdg = parent.join("xdg");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&xdg).unwrap();
        let status = Command::new("git")
            .arg("init")
            .current_dir(&root)
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE")
            .status()
            .unwrap();
        assert!(status.success());
        Self { parent, root, xdg }
    }

    fn init(&self) -> std::process::Output {
        self.init_in(&self.root)
    }

    fn init_in(&self, dir: &Path) -> std::process::Output {
        Command::new(env!("CARGO_BIN_EXE_licencify"))
            .arg("init")
            .current_dir(dir)
            .env("XDG_CONFIG_HOME", &self.xdg)
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
fn init_creates_only_shared_project_config_with_schema_url() {
    let project = Project::new();
    let output = project.init();
    assert!(output.status.success());

    let stdout = text(&output.stdout);
    let shared_config_path = project.root.join(".config/licencify/config.toml");
    assert!(
        stdout.contains(&format!(
            "Created project config: {}",
            shared_config_path.display()
        )),
        "stdout should report created config: {stdout}"
    );

    // Criteria 1: creates only the shared project config
    assert!(
        shared_config_path.exists(),
        "Shared project config should exist"
    );

    // Criteria 4: does not create the global config or local override
    let local_config_path = project.root.join(".config/licencify/config.local.toml");
    assert!(
        !local_config_path.exists(),
        "Local override must not be created"
    );

    let global_config_path = project.xdg.join("licencify/config.toml");
    assert!(
        !global_config_path.exists(),
        "Global config must not be created"
    );

    // Criteria 2: new file starts with #:schema comment and raw GitHub schema URL
    let contents = fs::read_to_string(&shared_config_path).unwrap();
    let expected_schema_prefix = format!("#:schema {EXPECTED_SCHEMA_URL}");
    assert!(
        contents.starts_with(&expected_schema_prefix),
        "File should start with schema comment and URL: {contents}"
    );

    let first_line = contents.lines().next().unwrap();
    assert_eq!(
        first_line, expected_schema_prefix,
        "First line must match schema URL"
    );
    assert!(contents.contains("[default]"));
}

#[test]
fn second_init_does_not_overwrite_existing_project_config() {
    let project = Project::new();
    let first_output = project.init();
    assert!(first_output.status.success());

    let shared_config_path = project.root.join(".config/licencify/config.toml");
    assert!(shared_config_path.exists());

    let custom_content = format!(
        "#:schema {EXPECTED_SCHEMA_URL}\n\n[default]\nauthor = \"Custom Author\"\nlicence = \"MIT\"\n"
    );
    fs::write(&shared_config_path, &custom_content).unwrap();

    // Criteria 3: second init does not overwrite an existing project config
    let second_output = project.init();
    assert!(second_output.status.success());

    let stdout = text(&second_output.stdout);
    assert!(
        stdout.contains(&format!(
            "Project config already exists: {}",
            shared_config_path.display()
        )),
        "stdout should report existing config: {stdout}"
    );

    let current_content = fs::read_to_string(&shared_config_path).unwrap();
    assert_eq!(
        current_content, custom_content,
        "Existing project config must be left unchanged"
    );

    assert!(
        !project.root.join(".config/licencify/config.local.toml").exists(),
        "Local override must not be created on second init"
    );
    assert!(
        !project.xdg.join("licencify/config.toml").exists(),
        "Global config must not be created on second init"
    );
}

#[test]
fn init_respects_prj_config_home() {
    let project = Project::new();
    let custom_config_home = project.parent.join("custom_config");

    let output = Command::new(env!("CARGO_BIN_EXE_licencify"))
        .arg("init")
        .current_dir(&project.root)
        .env("XDG_CONFIG_HOME", &project.xdg)
        .env("PRJ_CONFIG_HOME", &custom_config_home)
        .env_remove("PRJ_ROOT")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .output()
        .unwrap();

    assert!(output.status.success());

    let target_file = custom_config_home.join("licencify/config.toml");
    assert!(
        target_file.exists(),
        "Target config at PRJ_CONFIG_HOME must exist"
    );

    let content = fs::read_to_string(&target_file).unwrap();
    assert!(
        content.starts_with(&format!("#:schema {EXPECTED_SCHEMA_URL}")),
        "Config at PRJ_CONFIG_HOME must start with schema URL"
    );

    assert!(
        !project.root.join(".config/licencify/config.toml").exists(),
        "Default project config path must not be created when PRJ_CONFIG_HOME is set"
    );
    assert!(!project.xdg.join("licencify/config.toml").exists());
}

#[test]
fn init_from_subdirectory_creates_config_at_project_root() {
    let project = Project::new();
    let sub = project.root.join("some/nested/sub");
    fs::create_dir_all(&sub).unwrap();

    let output = project.init_in(&sub);
    assert!(output.status.success());

    let shared_config_path = project.root.join(".config/licencify/config.toml");
    assert!(
        shared_config_path.exists(),
        "Project root config must be created"
    );
    assert!(
        !sub.join(".config/licencify/config.toml").exists(),
        "Child config must not be created"
    );
}
