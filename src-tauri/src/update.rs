//! In-app updates from the GitHub releases of the project.
//! Check: the latest release through the GitHub API. Install: download the release tarball, verify its
//! SHA-256 against the published checksum, make sure the new binary starts, then hand over to the
//! package's install.sh in update mode (it keeps settings, shortcuts and autostart as they are).
use crate::i18n::{t, tf};
use crate::util::{data_home, run_status, which};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const REPO: &str = "giannoccarol/Activity-manager-linux";

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub newer: bool,
    pub name: String,
    pub notes: String,
    pub url: String,
    pub published: String,
    pub asset_url: String,
    pub sha_url: String,
    pub size: u64,
    /// false for package-managed or development builds: the update is only announced
    pub can_install: bool,
    pub reason: Option<String>,
}

/// Version of this build. GA_UPDATE_TEST_VERSION overrides it to test the update path.
pub fn current_version() -> String {
    std::env::var("GA_UPDATE_TEST_VERSION").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
}

fn parse(v: &str) -> Vec<u64> {
    v.trim_start_matches('v').split(|c: char| c == '.' || c == '-').take(3).map(|p| p.parse().unwrap_or(0)).collect()
}

/// true if `latest` is a higher version than `current` (major.minor.patch)
pub fn is_newer(latest: &str, current: &str) -> bool {
    parse(latest) > parse(current)
}

/// Installed per user by install.sh (in ~/.local/share/gestione-attivita/bin)? Only then the app
/// replaces itself; a package manager or a development build keeps control of its own files.
fn install_target() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = std::fs::canonicalize(&exe).unwrap_or(exe);
    let dir = data_home().join("gestione-attivita/bin");
    let dir = std::fs::canonicalize(&dir).unwrap_or(dir);
    if exe.starts_with(&dir) {
        Ok(exe)
    } else if exe.starts_with("/usr") || exe.starts_with("/opt") {
        Err(t("update.packageManaged"))
    } else {
        Err(t("update.devBuild"))
    }
}

fn curl_json(url: &str) -> Result<serde_json::Value, String> {
    if !which("curl") {
        return Err(t("update.noCurl"));
    }
    let (ok, out) = run_status(
        "curl",
        &["-fsSL", "--max-time", "20", "-H", "Accept: application/vnd.github+json", "-H", "User-Agent: gestione-attivita", url],
        Duration::from_secs(25),
    );
    if !ok || out.is_empty() {
        return Err(t("update.unreachable"));
    }
    serde_json::from_str(&out).map_err(|_| t("update.badResponse"))
}

/// Latest release on GitHub, compared with this build.
pub fn check() -> Result<UpdateInfo, String> {
    // GA_UPDATE_API points the check at another server (used to test the whole update path)
    let api = std::env::var("GA_UPDATE_API").ok().filter(|v| !v.is_empty()).unwrap_or_else(|| format!("https://api.github.com/repos/{REPO}/releases/latest"));
    let v = curl_json(&api)?;
    let tag = v["tag_name"].as_str().unwrap_or("").to_string();
    let latest = tag.trim_start_matches('v').to_string();
    if latest.is_empty() {
        return Err(t("update.badResponse"));
    }
    let current = current_version();
    let asset_name = format!("gestione-attivita-{latest}-linux-x86_64.tar.gz");
    let assets = v["assets"].as_array().cloned().unwrap_or_default();
    let find = |name: &str| assets.iter().find(|a| a["name"].as_str() == Some(name)).cloned();
    let asset = find(&asset_name);
    let sha = find(&format!("{asset_name}.sha256"));
    let (can_install, reason) = match (install_target(), &asset, &sha) {
        (Err(r), _, _) => (false, Some(r)),
        (Ok(_), None, _) | (Ok(_), _, None) => (false, Some(t("update.noAsset"))),
        _ if std::env::consts::ARCH != "x86_64" => (false, Some(t("update.noAsset"))),
        _ => (true, None),
    };
    Ok(UpdateInfo {
        newer: is_newer(&latest, &current),
        current,
        latest,
        name: v["name"].as_str().unwrap_or(&tag).to_string(),
        notes: v["body"].as_str().unwrap_or("").to_string(),
        url: v["html_url"].as_str().unwrap_or("").to_string(),
        published: v["published_at"].as_str().unwrap_or("").to_string(),
        asset_url: asset.as_ref().and_then(|a| a["browser_download_url"].as_str()).unwrap_or("").to_string(),
        sha_url: sha.as_ref().and_then(|a| a["browser_download_url"].as_str()).unwrap_or("").to_string(),
        size: asset.as_ref().and_then(|a| a["size"].as_u64()).unwrap_or(0),
        can_install,
        reason,
    })
}

fn download(url: &str, to: &Path, size: u64, progress: &dyn Fn(u64, u64)) -> Result<(), String> {
    let _ = std::fs::remove_file(to);
    let mut child = Command::new("curl")
        .args(["-fsSL", "--max-time", "900", "-H", "User-Agent: gestione-attivita", "-o"])
        .arg(to)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| t("update.noCurl"))?;
    let start = Instant::now();
    loop {
        if let Some(st) = child.try_wait().map_err(|e| e.to_string())? {
            if !st.success() {
                return Err(t("update.downloadFailed"));
            }
            break;
        }
        if start.elapsed() > Duration::from_secs(960) {
            let _ = child.kill();
            return Err(t("update.downloadFailed"));
        }
        progress(std::fs::metadata(to).map(|m| m.len()).unwrap_or(0), size);
        std::thread::sleep(Duration::from_millis(150));
    }
    progress(size, size);
    Ok(())
}

fn sha256_of(path: &Path) -> Result<String, String> {
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Runs `<bin> --version` (without the test overrides) and checks it prints `version`.
fn reports_version(bin: &Path, version: &str) -> bool {
    let child = Command::new(bin)
        .arg("--version")
        .env_remove("GA_UPDATE_TEST_VERSION")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = child else { return false };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(st)) if st.success() => break,
            Ok(Some(_)) | Err(_) => return false,
            Ok(None) if start.elapsed() > Duration::from_secs(10) => {
                let _ = child.kill();
                return false;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
        }
    }
    let mut out = String::new();
    if let Some(mut o) = child.stdout.take() {
        let _ = o.read_to_string(&mut out);
    }
    out.trim() == version
}

/// Phases reported to the interface while updating.
pub enum Step {
    Download(u64, u64),
    Verify,
    Install,
}

/// Downloads, verifies and starts the installation. On success install.sh closes this instance
/// and starts the new version (`show` = with the window open, otherwise in the system tray).
pub fn install(info: &UpdateInfo, show: bool, step: &dyn Fn(Step)) -> Result<(), String> {
    if !info.can_install {
        return Err(info.reason.clone().unwrap_or_else(|| t("update.noAsset")));
    }
    install_target()?;
    let dir = crate::util::home().join(".cache/gestione-attivita/update");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let tarball = dir.join("update.tar.gz");
    let shafile = dir.join("update.tar.gz.sha256");

    download(&info.asset_url, &tarball, info.size, &|d, total| step(Step::Download(d, total)))?;
    download(&info.sha_url, &shafile, 0, &|_, _| {})?;

    step(Step::Verify);
    let expected = std::fs::read_to_string(&shafile).unwrap_or_default().split_whitespace().next().unwrap_or("").to_lowercase();
    let actual = sha256_of(&tarball)?;
    if expected.len() != 64 || expected != actual {
        return Err(t("update.checksum"));
    }
    let (ok, _) = run_status("tar", &["-xzf", &tarball.to_string_lossy(), "-C", &dir.to_string_lossy()], Duration::from_secs(60));
    let pkg = dir.join(format!("gestione-attivita-{}-linux-x86_64", info.latest));
    let new_bin = pkg.join("gestione-attivita");
    if !ok || !new_bin.is_file() || !pkg.join("install.sh").is_file() {
        return Err(t("update.badPackage"));
    }
    // the new binary must start on this system (libraries, glibc) before replacing the current one
    if !reports_version(&new_bin, &info.latest) {
        return Err(tf("update.wontStart", &[("version", info.latest.clone())]));
    }

    step(Step::Install);
    Command::new("setsid")
        .arg("bash")
        .arg(pkg.join("install.sh"))
        .env("GA_UPDATE", if show { "show" } else { "tray" })
        .env_remove("GA_UPDATE_TEST_VERSION")
        .env_remove("GA_UPDATE_API")
        .current_dir(&pkg)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// `gestione-attivita --update` / `--check-update` from a terminal.
pub fn cli(install_too: bool) -> i32 {
    let info = match check() {
        Ok(i) => i,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    if !info.newer {
        println!("{}", tf("update.cliUpToDate", &[("version", info.current.clone())]));
        return 0;
    }
    println!("{}", tf("update.cliAvailable", &[("version", info.latest.clone()), ("current", info.current.clone())]));
    if !install_too {
        return 0;
    }
    let res = install(&info, false, &|s| match s {
        Step::Download(d, total) if total > 0 => {
            print!("\r{} {:>3}%", t("update.downloading"), d * 100 / total);
            let _ = std::io::Write::flush(&mut std::io::stdout());
        }
        Step::Verify => println!("\n{}", t("update.verifying")),
        Step::Install => println!("{}", t("update.installing")),
        _ => {}
    });
    match res {
        Ok(()) => {
            println!("{}", tf("update.cliStarted", &[("version", info.latest.clone())]));
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}
