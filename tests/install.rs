//! The plugin manifest, `scripts/install.sh` and the README config block.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

static COUNTER: AtomicUsize = AtomicUsize::new(0);

/// An install run that has not exited by then is hung.
const RUN_TIMEOUT: Duration = Duration::from_secs(10);

const DEFAULT_RELEASE_URL: &str = "https://github.com/santhoshsram/herdr-focus/releases/download";

/// (`uname -s`, `uname -m`, release asset name)
const MACHINES: [(&str, &str, &str); 3] = [
    ("Darwin", "arm64", "herdr-focus-macos-arm64"),
    ("Linux", "x86_64", "herdr-focus-linux-x86_64"),
    ("Linux", "aarch64", "herdr-focus-linux-arm64"),
];

const FAKE_CURL: &str = r#"#!/bin/sh
printf '%s\n' "$@" >> "@LOG@"
out=""
url=""
while [ $# -gt 0 ]; do
  case "$1" in
    -o|--output) out="$2"; shift 2 ;;
    -*) shift ;;
    *) url="$1"; shift ;;
  esac
done
src="@FIXTURES@/$(basename "$url")"
if [ ! -f "$src" ]; then
  echo "curl: (22) The requested URL returned error: 404" >&2
  exit 22
fi
if [ -n "$out" ]; then cp "$src" "$out"; else cat "$src"; fi
"#;

const FAKE_UNAME: &str = r#"#!/bin/sh
case "$1" in
  -s) echo "@S@" ;;
  -m) echo "@M@" ;;
  -sm|-ms) echo "@S@ @M@" ;;
  *) echo "@S@" ;;
esac
"#;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn make_executable(path: &Path) {
    let mut perms = fs::metadata(path).expect("stat script").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("chmod script");
}

/// Result of one `sh scripts/install.sh` run.
struct Run {
    code: Option<i32>,
    stderr: String,
}

/// A temp world: a HOME, a plugin folder holding a copy of `scripts/` and
/// the manifest, a fake `uname` and `curl` ahead of the real PATH, and a
/// folder of fixture release assets.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new(tag: &str) -> Self {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!(
            "herdr-focus-install-{}-{}-{}",
            std::process::id(),
            n,
            tag
        ));
        let _ = fs::remove_dir_all(&root);
        for dir in ["home", "plugin/scripts", "fakebin", "fixtures"] {
            fs::create_dir_all(root.join(dir)).expect("create sandbox dir");
        }
        let sandbox = Self { root };
        let repo = repo_root();
        fs::copy(
            repo.join("scripts/install.sh"),
            sandbox.plugin().join("scripts/install.sh"),
        )
        .expect("copy scripts/install.sh");
        fs::copy(
            repo.join("herdr-plugin.toml"),
            sandbox.plugin().join("herdr-plugin.toml"),
        )
        .expect("copy herdr-plugin.toml");
        sandbox.write_curl();
        sandbox
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    fn plugin(&self) -> PathBuf {
        self.root.join("plugin")
    }

    fn fixtures(&self) -> PathBuf {
        self.root.join("fixtures")
    }

    fn curl_log(&self) -> PathBuf {
        self.root.join("curl.log")
    }

    fn installed(&self) -> PathBuf {
        self.home()
            .join(".local/state/herdr/plugins/focus/herdr-focus")
    }

    fn plugin_copy(&self) -> PathBuf {
        self.plugin().join("bin/herdr-focus")
    }

    fn write_curl(&self) {
        let script = FAKE_CURL
            .replace("@LOG@", self.curl_log().to_str().expect("utf8 path"))
            .replace("@FIXTURES@", self.fixtures().to_str().expect("utf8 path"));
        let path = self.root.join("fakebin/curl");
        fs::write(&path, script).expect("write fake curl");
        make_executable(&path);
    }

    fn fake_uname(&self, sys: &str, machine: &str) {
        let script = FAKE_UNAME.replace("@S@", sys).replace("@M@", machine);
        let path = self.root.join("fakebin/uname");
        fs::write(&path, script).expect("write fake uname");
        make_executable(&path);
    }

    fn serve(&self, asset: &str, contents: &str) {
        fs::write(self.fixtures().join(asset), contents).expect("write fixture asset");
    }

    fn curl_args(&self) -> Vec<String> {
        fs::read_to_string(self.curl_log())
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    /// The manifest's own `version`, which names the release to download.
    fn version(&self) -> String {
        let text = fs::read_to_string(self.plugin().join("herdr-plugin.toml"))
            .expect("read herdr-plugin.toml");
        let doc = parse_manifest(&text);
        unquote(doc.top("version").expect("manifest has a version")).to_string()
    }

    /// Runs `sh scripts/install.sh` from the plugin folder.
    fn run(&self, release_url: Option<&str>) -> Run {
        let outside = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".to_string());
        let path = format!(
            "{}:{}:/usr/bin:/bin",
            self.root.join("fakebin").display(),
            outside
        );
        let stderr_path = self.root.join("stderr.txt");
        let stderr_file = fs::File::create(&stderr_path).expect("create stderr file");
        let mut cmd = Command::new("/bin/sh");
        cmd.arg("scripts/install.sh")
            .current_dir(self.plugin())
            .env("HOME", self.home())
            .env("PATH", path)
            .env_remove("HERDR_FOCUS_RELEASE_URL")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(stderr_file));
        if let Some(url) = release_url {
            cmd.env("HERDR_FOCUS_RELEASE_URL", url);
        }
        let mut child = cmd.spawn().expect("spawn sh scripts/install.sh");
        let start = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().expect("poll install.sh") {
                break status;
            }
            if start.elapsed() > RUN_TIMEOUT {
                let _ = child.kill();
                let _ = child.wait();
                panic!("install.sh did not exit within {RUN_TIMEOUT:?}");
            }
            thread::sleep(Duration::from_millis(20));
        };
        Run {
            code: status.code(),
            stderr: fs::read_to_string(&stderr_path).unwrap_or_default(),
        }
    }

    fn assert_nothing_copied(&self) {
        assert!(
            !self.installed().exists(),
            "nothing may be copied to the state folder"
        );
        assert!(
            !self.plugin_copy().exists(),
            "nothing may be placed in the plugin folder"
        );
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A manifest as top-level keys plus its tables, each with raw `key = value` pairs.
struct Manifest {
    top: Vec<(String, String)>,
    tables: Vec<(String, Vec<(String, String)>)>,
}

impl Manifest {
    fn top(&self, key: &str) -> Option<&str> {
        self.top
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    fn tables(&self, name: &str) -> Vec<&Vec<(String, String)>> {
        self.tables
            .iter()
            .filter(|(n, _)| n == name)
            .map(|(_, pairs)| pairs)
            .collect()
    }
}

fn get<'a>(pairs: &'a [(String, String)], key: &str) -> Option<&'a str> {
    pairs
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
}

fn unquote(value: &str) -> &str {
    value.trim().trim_matches(['"', '\''])
}

fn parse_manifest(text: &str) -> Manifest {
    let mut top = Vec::new();
    let mut tables: Vec<(String, Vec<(String, String)>)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line.strip_prefix("[[").and_then(|l| l.strip_suffix("]]")) {
            tables.push((name.trim().to_string(), Vec::new()));
        } else if line.starts_with('[') {
            tables.push((line.trim_matches(['[', ']']).trim().to_string(), Vec::new()));
        } else if let Some((key, value)) = line.split_once('=') {
            let pair = (key.trim().to_string(), value.trim().to_string());
            match tables.last_mut() {
                Some((_, pairs)) => pairs.push(pair),
                None => top.push(pair),
            }
        }
    }
    Manifest { top, tables }
}

#[test]
fn install_refuses_unknown_machine() {
    for (sys, machine) in [
        ("FreeBSD", "amd64"),
        ("Linux", "riscv64"),
        ("Darwin", "x86_64"),
    ] {
        let sandbox = Sandbox::new("unknown");
        sandbox.fake_uname(sys, machine);
        for (_, _, asset) in MACHINES {
            sandbox.serve(asset, "fake binary");
        }

        let run = sandbox.run(Some("http://release.invalid/download"));

        assert!(
            run.code.is_some_and(|c| c != 0),
            "{sys} {machine}: install.sh must exit non-zero, got {:?}",
            run.code
        );
        let lines: Vec<&str> = run
            .stderr
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();
        assert_eq!(
            lines.len(),
            1,
            "{sys} {machine}: expected exactly one stderr line, got {:?}",
            run.stderr
        );
        sandbox.assert_nothing_copied();
    }
}

#[test]
fn install_copies_binary() {
    let base = "http://release.invalid/download";
    for (sys, machine, asset) in MACHINES {
        let sandbox = Sandbox::new("copies");
        sandbox.fake_uname(sys, machine);
        for (_, _, other) in MACHINES {
            sandbox.serve(other, &format!("fake binary for {other}\n"));
        }

        let run = sandbox.run(Some(base));

        assert_eq!(
            run.code,
            Some(0),
            "{sys} {machine}: install.sh must succeed, stderr: {}",
            run.stderr
        );
        let want = format!("fake binary for {asset}\n");
        let installed = sandbox.installed();
        assert_eq!(
            fs::read_to_string(&installed).expect("installed binary exists"),
            want,
            "{sys} {machine}: wrong binary installed"
        );
        let mode = fs::metadata(&installed)
            .expect("stat installed")
            .permissions()
            .mode();
        assert!(mode & 0o111 != 0, "installed binary must be executable");
        assert_eq!(
            fs::read_to_string(sandbox.plugin_copy()).expect("plugin folder copy exists"),
            want,
            "{sys} {machine}: wrong binary in the plugin folder"
        );
        let url = format!("{base}/v{}/{asset}", sandbox.version());
        assert!(
            sandbox.curl_args().contains(&url),
            "{sys} {machine}: curl must fetch {url}, got {:?}",
            sandbox.curl_args()
        );
    }

    let sandbox = Sandbox::new("default-base");
    sandbox.fake_uname("Linux", "x86_64");
    sandbox.serve("herdr-focus-linux-x86_64", "fake binary");
    let run = sandbox.run(None);
    assert_eq!(run.code, Some(0), "default base: stderr: {}", run.stderr);
    let url = format!(
        "{DEFAULT_RELEASE_URL}/v{}/herdr-focus-linux-x86_64",
        sandbox.version()
    );
    assert!(
        sandbox.curl_args().contains(&url),
        "default base: curl must fetch {url}, got {:?}",
        sandbox.curl_args()
    );
}

#[test]
fn install_failed_download() {
    for (sys, machine, _) in MACHINES {
        let sandbox = Sandbox::new("download-fails");
        sandbox.fake_uname(sys, machine);
        // No fixture is served, so the fake curl fails like a 404.

        let run = sandbox.run(Some("http://release.invalid/download"));

        assert!(
            run.code.is_some_and(|c| c != 0),
            "{sys} {machine}: install.sh must exit non-zero, got {:?}",
            run.code
        );
        assert!(
            !sandbox.curl_args().is_empty(),
            "{sys} {machine}: the download must have been tried"
        );
        sandbox.assert_nothing_copied();
    }
}

#[test]
fn manifest_shape() {
    let text =
        fs::read_to_string(repo_root().join("herdr-plugin.toml")).expect("read herdr-plugin.toml");
    let doc = parse_manifest(&text);

    assert_eq!(doc.top("id").map(unquote), Some("focus"));
    let platforms = doc.top("platforms").expect("platforms is set");
    assert!(platforms.contains("\"macos\""), "platforms: {platforms}");
    assert!(platforms.contains("\"linux\""), "platforms: {platforms}");

    assert!(
        doc.tables("actions")
            .iter()
            .any(|a| get(a, "id").map(unquote) == Some("open")),
        "an [[actions]] with id \"open\" is required"
    );
    assert!(
        doc.tables("panes").iter().any(|p| {
            get(p, "id").map(unquote) == Some("panel")
                && get(p, "placement").map(unquote) == Some("popup")
        }),
        "a [[panes]] with id \"panel\" and placement \"popup\" is required"
    );
    let builds = doc.tables("build");
    assert_eq!(builds.len(), 1, "exactly one [[build]] is required");
    assert!(
        get(builds[0], "command").is_some_and(|c| c.contains("scripts/install.sh")),
        "the build command must run scripts/install.sh"
    );
}

#[test]
fn readme_has_config_block() {
    let readme = fs::read_to_string(repo_root().join("README.md")).expect("read README.md");
    assert!(
        readme.contains("~/.local/state/herdr/plugins/focus/herdr-focus status"),
        "README must hold the tab bar command"
    );
    assert!(
        readme.contains("herdr plugin install santhoshsram/herdr-focus"),
        "README must hold the install command"
    );
}

#[test]
fn readme_inline_tables_on_one_line() {
    let readme = fs::read_to_string(repo_root().join("README.md")).expect("read README.md");
    let block: Vec<&str> = readme
        .lines()
        .skip_while(|l| l.trim() != "```toml")
        .skip(1)
        .take_while(|l| l.trim() != "```")
        .collect();
    assert!(!block.is_empty(), "README must hold a toml block");
    assert!(
        block.iter().any(|l| l.contains('{')),
        "the toml block must hold an inline table"
    );

    for line in block {
        let mut depth = 0i32;
        for c in line.chars() {
            match c {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
            assert!(depth >= 0, "inline table closes before it opens: {line:?}");
        }
        assert_eq!(depth, 0, "inline table is not closed on its line: {line:?}");
    }
}
