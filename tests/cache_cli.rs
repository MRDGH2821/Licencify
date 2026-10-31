use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

struct MockSpdxServer {
    addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl MockSpdxServer {
    fn new(index_body: Arc<std::sync::RwLock<String>>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_clone = shutdown.clone();

        listener.set_nonblocking(true).expect("set nonblocking");

        let handle = thread::spawn(move || {
            while !shutdown_clone.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((mut stream, _)) => {
                        Self::handle_connection(&mut stream, &index_body);
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(_) => break,
                }
            }
        });

        Self {
            addr,
            shutdown,
            handle: Some(handle),
        }
    }

    fn url(&self) -> String {
        format!("http://{}", self.addr)
    }

    fn handle_connection(stream: &mut TcpStream, index_body: &Arc<std::sync::RwLock<String>>) {
        let mut buffer = [0u8; 2048];
        let bytes_read = stream.read(&mut buffer).unwrap_or(0);
        if bytes_read == 0 {
            return;
        }

        let request = String::from_utf8_lossy(&buffer[..bytes_read]);
        let mut lines = request.lines();
        let request_line = lines.next().unwrap_or("");
        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return;
        }

        let path = parts[1];
        let (status, body) = match path {
            "/licenses.json" => {
                let current_index = index_body.read().unwrap().clone();
                ("200 OK", current_index)
            }
            "/MIT.json" => (
                "200 OK",
                r#"{"licenseId":"MIT","name":"MIT License","licenseText":"MIT test body","licenseTextHtml":"<p>MIT test html</p>"}"#.to_string(),
            ),
            "/Apache-2.0.json" => (
                "200 OK",
                r#"{"licenseId":"Apache-2.0","name":"Apache License 2.0","licenseText":"Apache test body","licenseTextHtml":"<p>Apache test html</p>"}"#.to_string(),
            ),
            _ => ("404 Not Found", r#"{"error":"not found"}"#.to_string()),
        };

        let response = format!(
            "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status,
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.flush();
    }
}

impl Drop for MockSpdxServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

struct TestEnv {
    temp_dir: PathBuf,
    cache_home: PathBuf,
    project_root: PathBuf,
}

impl TestEnv {
    fn new() -> Self {
        let temp_dir = std::env::temp_dir().join(format!(
            "licencify-cache-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cache_home = temp_dir.join("xdg_cache");
        let project_root = temp_dir.join("project");

        fs::create_dir_all(&cache_home).unwrap();
        fs::create_dir_all(&project_root).unwrap();

        Self {
            temp_dir,
            cache_home,
            project_root,
        }
    }

    fn spdx_cache_dir(&self) -> PathBuf {
        self.cache_home.join("licencify").join("SPDX-Cache")
    }

    fn run(&self, args: &[&str], spdx_url: Option<&str>) -> std::process::Output {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_licencify"));
        cmd.current_dir(&self.project_root)
            .env("XDG_CACHE_HOME", &self.cache_home)
            .env_remove("PRJ_ROOT")
            .env_remove("PRJ_CONFIG_HOME")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE");
        if let Some(url) = spdx_url {
            cmd.env("LICENCIFY_SPDX_URL", url);
        }
        cmd.args(args);
        cmd.output().expect("failed to execute licencify")
    }
}

impl Drop for TestEnv {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

fn stdout_text(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_text(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn bare_cache_prints_directory_file_count_and_size() {
    let env = TestEnv::new();
    let spdx_dir = env.spdx_cache_dir();

    // 1. When cache directory does not exist yet
    let out = env.run(&["cache"], None);
    assert!(out.status.success(), "{}", stderr_text(&out));
    let text = stdout_text(&out);
    assert!(
        text.contains(&format!("Cache directory: {}", spdx_dir.display())),
        "{text}"
    );
    assert!(text.contains("Cached files: 0"), "{text}");
    assert!(text.contains("Cache size: 0 bytes"), "{text}");

    // 2. When cache directory has files
    fs::create_dir_all(&spdx_dir).unwrap();
    let test_file1 = spdx_dir.join("one.json");
    let test_file2 = spdx_dir.join("two.json");
    fs::write(&test_file1, "12345").unwrap(); // 5 bytes
    fs::write(&test_file2, "1234567").unwrap(); // 7 bytes
    let total_bytes = 5 + 7;

    let out = env.run(&["cache"], None);
    assert!(out.status.success(), "{}", stderr_text(&out));
    let text = stdout_text(&out);
    assert!(
        text.contains(&format!("Cache directory: {}", spdx_dir.display())),
        "{text}"
    );
    assert!(text.contains("Cached files: 2"), "{text}");
    assert!(
        text.contains(&format!("Cache size: {total_bytes} bytes")),
        "{text}"
    );
}

#[test]
fn cache_update_refreshes_index_and_does_not_download_bodies() {
    let env = TestEnv::new();
    let spdx_dir = env.spdx_cache_dir();

    let initial_index =
        r#"{"licenseListVersion":"3.28.0","licenses":[{"licenseId":"MIT","name":"MIT License"}]}"#
            .to_string();
    let index_holder = Arc::new(std::sync::RwLock::new(initial_index));
    let server = MockSpdxServer::new(index_holder.clone());

    // 1. Run cache update: downloads licenses.json only
    let out = env.run(&["cache", "update"], Some(&server.url()));
    assert!(out.status.success(), "{}", stderr_text(&out));
    assert!(
        stdout_text(&out).contains("Updated SPDX licence index"),
        "{}",
        stdout_text(&out)
    );

    let licenses_json = spdx_dir.join("licenses.json");
    assert!(licenses_json.exists(), "licenses.json must exist in cache");
    let content = fs::read_to_string(&licenses_json).unwrap();
    assert!(content.contains("MIT"));

    // Ensure NO licence bodies were downloaded
    let entries: Vec<PathBuf> = fs::read_dir(&spdx_dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    assert_eq!(
        entries,
        vec![licenses_json.clone()],
        "cache update must only download licenses.json and not bodies"
    );

    // 2. Refresh index with updated content on server
    let updated_index = r#"{"licenseListVersion":"3.29.0","licenses":[{"licenseId":"MIT","name":"MIT License"},{"licenseId":"Apache-2.0","name":"Apache License 2.0"}]}"#.to_string();
    *index_holder.write().unwrap() = updated_index;

    let out2 = env.run(&["cache", "update"], Some(&server.url()));
    assert!(out2.status.success(), "{}", stderr_text(&out2));

    let content2 = fs::read_to_string(&licenses_json).unwrap();
    assert!(content2.contains("Apache-2.0"));
    assert!(content2.contains("3.29.0"));

    // Still no bodies downloaded
    let entries2: Vec<PathBuf> = fs::read_dir(&spdx_dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    assert_eq!(entries2, vec![licenses_json]);
}

#[test]
fn cache_fetch_one_downloads_id_and_does_not_refetch_cached_copy() {
    let env = TestEnv::new();
    let spdx_dir = env.spdx_cache_dir();

    let index = r#"{"licenses":[{"licenseId":"MIT","name":"MIT License"},{"licenseId":"Apache-2.0","name":"Apache 2.0"}]}"#.to_string();
    let server = MockSpdxServer::new(Arc::new(std::sync::RwLock::new(index)));

    // 1. Fetch MIT: downloads that one id
    let out = env.run(&["cache", "fetch", "MIT"], Some(&server.url()));
    assert!(out.status.success(), "{}", stderr_text(&out));
    assert!(stdout_text(&out).contains("Fetched MIT"));

    let mit_json = spdx_dir.join("MIT.json");
    assert!(mit_json.exists(), "MIT.json must be cached");
    let content = fs::read_to_string(&mit_json).unwrap();
    assert!(content.contains("MIT test body"));

    // 2. Overwrite MIT.json with a custom marker
    let custom_marker =
        r#"{"licenseId":"MIT","name":"MIT License","licenseText":"custom marker content"}"#;
    fs::write(&mit_json, custom_marker).unwrap();

    // 3. Run cache fetch MIT again: must NOT refetch
    let out2 = env.run(&["cache", "fetch", "MIT"], Some(&server.url()));
    assert!(out2.status.success(), "{}", stderr_text(&out2));
    assert!(stdout_text(&out2).contains("MIT is already cached"));

    let content_after = fs::read_to_string(&mit_json).unwrap();
    assert_eq!(
        content_after, custom_marker,
        "cached copy must not be refetched or overwritten"
    );

    // 4. Fetching unknown ID fails
    let out3 = env.run(&["cache", "fetch", "NONEXISTENT-XYZ"], Some(&server.url()));
    assert!(!out3.status.success(), "fetching unknown id must fail");
    assert!(stderr_text(&out3).contains("Unknown license ID"));
}

#[test]
fn cache_fetch_no_id_downloads_all_uncached_and_leaves_cached_in_place() {
    let env = TestEnv::new();
    let spdx_dir = env.spdx_cache_dir();
    fs::create_dir_all(&spdx_dir).unwrap();

    // Cache an index with 2 licenses
    let index = r#"{"licenses":[{"licenseId":"MIT","name":"MIT License"},{"licenseId":"Apache-2.0","name":"Apache License 2.0"}]}"#;
    fs::write(spdx_dir.join("licenses.json"), index).unwrap();

    // Pre-cache MIT.json with custom content
    let mit_marker = r#"{"licenseId":"MIT","name":"MIT License","licenseText":"do not touch"}"#;
    fs::write(spdx_dir.join("MIT.json"), mit_marker).unwrap();

    let server = MockSpdxServer::new(Arc::new(std::sync::RwLock::new(index.to_string())));

    // Run cache fetch with no id
    let out = env.run(&["cache", "fetch"], Some(&server.url()));
    assert!(out.status.success(), "{}", stderr_text(&out));
    let text = stdout_text(&out);
    assert!(text.contains("1 fetched, 1 skipped (already cached)"));

    // Apache-2.0 must be downloaded
    let apache_json = spdx_dir.join("Apache-2.0.json");
    assert!(apache_json.exists(), "Apache-2.0.json must be downloaded");
    let apache_content = fs::read_to_string(&apache_json).unwrap();
    assert!(apache_content.contains("Apache test body"));

    // MIT must remain untouched
    let mit_content = fs::read_to_string(spdx_dir.join("MIT.json")).unwrap();
    assert_eq!(
        mit_content, mit_marker,
        "pre-cached detail must remain untouched"
    );
}

#[test]
fn cache_update_and_fetch_do_not_refresh_each_other() {
    let env = TestEnv::new();
    let spdx_dir = env.spdx_cache_dir();
    fs::create_dir_all(&spdx_dir).unwrap();

    let index_v1 =
        r#"{"licenseListVersion":"v1","licenses":[{"licenseId":"MIT","name":"MIT License"}]}"#;
    fs::write(spdx_dir.join("licenses.json"), index_v1).unwrap();

    let server_index =
        r#"{"licenseListVersion":"v2","licenses":[{"licenseId":"MIT","name":"MIT License"}]}"#;
    let server = MockSpdxServer::new(Arc::new(std::sync::RwLock::new(server_index.to_string())));

    // 1. cache fetch MIT must NOT refresh licenses.json
    let out = env.run(&["cache", "fetch", "MIT"], Some(&server.url()));
    assert!(out.status.success(), "{}", stderr_text(&out));

    let index_after_fetch = fs::read_to_string(spdx_dir.join("licenses.json")).unwrap();
    assert_eq!(
        index_after_fetch, index_v1,
        "cache fetch must not refresh licenses.json"
    );

    // 2. cache update must NOT download bodies
    let out2 = env.run(&["cache", "update"], Some(&server.url()));
    assert!(out2.status.success(), "{}", stderr_text(&out2));

    let index_after_update = fs::read_to_string(spdx_dir.join("licenses.json")).unwrap();
    assert!(
        index_after_update.contains("v2"),
        "cache update must refresh licenses.json"
    );

    // Ensure no additional bodies were downloaded during update
    assert!(spdx_dir.join("MIT.json").exists());
    assert!(!spdx_dir.join("Apache-2.0.json").exists());
}

#[test]
fn cache_clear_removes_only_spdx_cache_directory() {
    let env = TestEnv::new();
    let spdx_dir = env.spdx_cache_dir();
    let licencify_parent = spdx_dir.parent().unwrap();

    // Populate cache directory
    fs::create_dir_all(&spdx_dir).unwrap();
    fs::write(spdx_dir.join("MIT.json"), "{}").unwrap();
    fs::write(spdx_dir.join("licenses.json"), "{}").unwrap();

    // Populate a file in the parent licencify directory
    let sibling_file = licencify_parent.join("unrelated_config.toml");
    fs::write(&sibling_file, "key = 'value'").unwrap();

    // Run cache clear
    let out = env.run(&["cache", "clear"], None);
    assert!(out.status.success(), "{}", stderr_text(&out));
    let text = stdout_text(&out);
    assert!(text.contains("Cleared 2 cached SPDX responses from"));

    // spdx_dir must be deleted
    assert!(
        !spdx_dir.exists(),
        "SPDX cache directory must be removed by cache clear"
    );

    // sibling file in parent directory must still exist
    assert!(
        sibling_file.exists(),
        "cache clear must remove only the SPDX cache directory, not parent directory contents"
    );
}
