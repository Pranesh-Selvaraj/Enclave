//! Self-update via GitHub Releases — one mechanism for desktop + Android.
//!
//! `check_for_update` hits the GitHub API for the latest release tag and
//! compares it with the installed version; `download_update` streams the
//! platform installer/APK into the app cache dir (emitting `update-progress`
//! events); `install_update` launches it: silent NSIS over-install on
//! Windows, `open` on macOS, the new AppImage on Linux, and the Android
//! package installer via the Kotlin `UpdaterPlugin` on Android. All paths
//! over-install — user data is kept, no uninstall needed.

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::sync::Mutex;
use tauri::Emitter;
use tauri::Manager;

const REPO: &str = "Pranesh-Selvaraj/Enclave";
const UA: &str = concat!("enclave-updater/", env!("CARGO_PKG_VERSION"));

#[derive(Clone, serde::Serialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub notes: String,
    pub asset_name: Option<String>,
    pub asset_url: Option<String>,
    pub asset_size: Option<u64>,
}

#[derive(Clone)]
struct UpdateAsset {
    name: String,
    url: String,
    size: u64,
    digest: Option<String>,
}

/// The asset selected by the last check plus the path of the one download
/// that passed verification. The download command takes no URL from the
/// frontend — it streams exactly what GitHub advertised — and install only
/// runs a file this process verified.
#[derive(Default)]
pub struct UpdateStateInner {
    pending: Option<UpdateAsset>,
    verified_path: Option<String>,
}

#[derive(Default)]
pub struct UpdateState(pub Mutex<UpdateStateInner>);

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    body: Option<String>,
    assets: Vec<GhAsset>,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    /// GitHub's `sha256:<hex>` asset digest (optional in older API responses).
    digest: Option<String>,
}

/// `sha256:<hex>` → lowercase hex. Any other scheme is refused (fail closed).
fn expected_sha256(digest: &str) -> Result<String, String> {
    let hex = digest
        .strip_prefix("sha256:")
        .ok_or_else(|| format!("unsupported update digest scheme: {digest}"))?;
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("malformed sha256 digest: {digest}"));
    }
    Ok(hex.to_ascii_lowercase())
}

#[cfg(test)]
fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// "1.2.0" / "v1.2.0" → [1, 2, 0]. Only the numeric version core is parsed:
/// prerelease (`-rc.1`) and build (`+build5`) suffixes are ignored, so
/// `1.2.0-rc.1` compares equal to `1.2.0` instead of looking newer because of
/// the extra numeric component.
fn parse_version(v: &str) -> Vec<u32> {
    let start = v.find(|c: char| c.is_ascii_digit()).unwrap_or(v.len());
    v[start..]
        .split(['-', '+'])
        .next()
        .unwrap_or("")
        .split(|c: char| !c.is_ascii_digit())
        .filter_map(|p| p.parse::<u32>().ok())
        .collect()
}

fn is_newer(latest: &str, current: &str) -> bool {
    let l = parse_version(latest);
    let c = parse_version(current);
    let n = l.len().max(c.len());
    for i in 0..n {
        match l
            .get(i)
            .copied()
            .unwrap_or(0)
            .cmp(&c.get(i).copied().unwrap_or(0))
        {
            std::cmp::Ordering::Greater => return true,
            std::cmp::Ordering::Less => return false,
            std::cmp::Ordering::Equal => {}
        }
    }
    false
}

fn platform_suffixes() -> &'static [&'static str] {
    if cfg!(target_os = "android") {
        &[".apk"]
    } else if cfg!(target_os = "windows") {
        // NSIS exe, not MSI: currentUser over-install works while running and
        // the installer kills the app in silent mode (see installer.nsi).
        &[".exe"]
    } else if cfg!(target_os = "macos") {
        &[".dmg"]
    } else {
        // AppImage runs anywhere (even deb-installed machines), so it is the
        // update vehicle on Linux.
        &[".appimage"]
    }
}

fn pick_asset<'a>(assets: &'a [GhAsset]) -> Option<&'a GhAsset> {
    let suffixes = platform_suffixes();
    // Skip the raw unsigned APK the CI glob also uploads — it can't install
    // over the signed app (signature mismatch).
    assets.iter().find(|a| {
        let n = a.name.to_ascii_lowercase();
        !n.contains("unsigned") && suffixes.iter().any(|s| n.ends_with(s))
    })
}

// ── Commands ────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[tauri::command]
pub async fn check_for_update(state: tauri::State<'_, UpdateState>) -> Result<UpdateInfo, String> {
    let current = env!("CARGO_PKG_VERSION").to_string();
    let json = tauri::async_runtime::spawn_blocking(move || {
        let mut resp = ureq::get(&format!("https://api.github.com/repos/{REPO}/releases/latest"))
            .header("User-Agent", UA)
            .header("Accept", "application/vnd.github+json")
            .call()
            .map_err(|e| e.to_string())?;
        resp.body_mut().read_to_string().map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    let release: GhRelease = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    let latest = release.tag_name.trim_start_matches('v').to_string();
    let asset = pick_asset(&release.assets);
    {
        let mut guard = state.0.lock().map_err(|_| "updater state poisoned".to_string())?;
        guard.pending = asset.map(|a| UpdateAsset {
            name: a.name.clone(),
            url: a.browser_download_url.clone(),
            size: a.size,
            digest: a.digest.clone(),
        });
        guard.verified_path = None;
    }
    Ok(UpdateInfo {
        current_version: current.clone(),
        latest_version: latest.clone(),
        update_available: is_newer(&latest, &current),
        notes: release.body.unwrap_or_default(),
        asset_name: asset.map(|a| a.name.clone()),
        asset_url: asset.map(|a| a.browser_download_url.clone()),
        asset_size: asset.map(|a| a.size),
    })
}

/// Streams the asset chosen by the last `check_for_update` into the app cache
/// dir; emits `update-progress` events ({received, total, percent}), verifies
/// the SHA-256 digest GitHub reported for the asset, and returns the local
/// path. A digest mismatch deletes the file and aborts.
#[tauri::command]
pub async fn download_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, UpdateState>,
) -> Result<String, String> {
    let asset = {
        let guard = state.0.lock().map_err(|_| "updater state poisoned".to_string())?;
        guard.pending.clone().ok_or("Run an update check first")?
    };
    let expected = asset.digest.as_deref().map(expected_sha256).transpose()?;

    let cache_dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    let dest = cache_dir.join(sanitize_filename(&asset.name));
    let dest_clone = dest.clone();
    let app_clone = app.clone();
    let url = asset.url.clone();
    let expected_size = asset.size;

    let verified = tauri::async_runtime::spawn_blocking(move || {
        let resp = ureq::get(&url)
            .header("User-Agent", UA)
            .call()
            .map_err(|e| e.to_string())?;
        let total = resp
            .headers()
            .get("content-length")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(0);
        let mut body = resp.into_body();
        let mut reader = body.as_reader();
        let mut out = std::fs::File::create(&dest_clone).map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 64 * 1024];
        let mut received = 0u64;
        let mut last_pct = u32::MAX;
        loop {
            let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            hasher.update(&buf[..n]);
            received += n as u64;
            if total > 0 {
                let pct = (received * 100 / total) as u32;
                if pct != last_pct {
                    last_pct = pct;
                    let _ = app_clone.emit(
                        "update-progress",
                        serde_json::json!({ "received": received, "total": total, "percent": pct }),
                    );
                }
            }
        }
        drop(out);

        let actual: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if let Some(expected) = &expected {
            if !actual.eq_ignore_ascii_case(expected) {
                let _ = std::fs::remove_file(&dest_clone);
                return Err("Update failed its integrity check (sha256 mismatch) — the download was discarded".to_string());
            }
        }
        // A short/truncated body must not be installed even when no digest is
        // advertised; GitHub always reports the asset size.
        if expected_size > 0 && received != expected_size {
            let _ = std::fs::remove_file(&dest_clone);
            return Err(format!(
                "Update size mismatch ({received} bytes received, {expected_size} expected) — the download was discarded"
            ));
        }
        Ok::<String, String>(dest_clone.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())??;

    {
        let mut guard = state.0.lock().map_err(|_| "updater state poisoned".to_string())?;
        guard.verified_path = Some(verified.clone());
    }
    Ok(verified)
}

/// Launches the downloaded installer/APK. Over-installs — no uninstall, user
/// data is kept on every platform.
#[tauri::command]
pub async fn install_update(
    app: tauri::AppHandle,
    state: tauri::State<'_, UpdateState>,
    path: String,
) -> Result<(), String> {
    // Only the file this process downloaded and verified may be executed.
    {
        let guard = state.0.lock().map_err(|_| "updater state poisoned".to_string())?;
        if guard.verified_path.as_deref() != Some(path.as_str()) {
            return Err("Refusing to install an update that was not verified by this session".into());
        }
    }
    #[cfg(target_os = "android")]
    {
        let plugin_state = app.state::<UpdaterState>();
        plugin_state
            .0
            .run_mobile_plugin::<serde_json::Value>("installApk", serde_json::json!({ "path": path }))
            .map(|_| ())
            .map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        let path = std::path::PathBuf::from(path);
        #[cfg(target_os = "windows")]
        {
            // Silent NSIS over-install; the tauri installer template kills
            // the running app in silent mode and replaces it in place.
            std::process::Command::new(&path)
                .arg("/S")
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(target_os = "macos")]
        {
            // Opens the DMG in Finder — dragging onto Applications replaces
            // the app and keeps the data container.
            std::process::Command::new("open")
                .arg(&path)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| e.to_string())?;
            // If launched from an AppImage, swap the downloaded file in so the
            // desktop entry runs the new version next launch.
            // ponytail: only the AppImage case is handled; deb installs just
            // run the downloaded AppImage directly (still the new version).
            if let Ok(installed) = std::env::var("APPIMAGE") {
                let _ = std::fs::copy(&path, installed);
            }
            std::process::Command::new(&path)
                .spawn()
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Err("Updates are not supported on this platform".into())
        }
    }
}

fn sanitize_filename(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
        .collect();
    if safe.is_empty() {
        "update.bin".into()
    } else {
        safe
    }
}

// ── Android bridge (Kotlin UpdaterPlugin) ───────────────────────────────────

/// Managed on Android so `install_update` can reach the Kotlin plugin.
#[cfg(target_os = "android")]
pub struct UpdaterState(pub tauri::plugin::PluginHandle<tauri::Wry>);

pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri::plugin::Builder::new("enclave-updater")
        .setup(|_app, _api| {
            _app.manage(UpdateState::default());
            #[cfg(target_os = "android")]
            {
                // Kotlin class: com.enclave.app.UpdaterPlugin (see
                // gen/android/app/src/main/java/com/enclave/app/UpdaterPlugin.kt)
                let handle = _api.register_android_plugin("com.enclave.app", "UpdaterPlugin")?;
                _app.manage(UpdaterState(handle));
            }
            Ok(())
        })
        .build()
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_compare_is_numeric_not_lexicographic() {
        assert!(is_newer("1.3.0", "1.2.0"));
        assert!(is_newer("1.2.1", "1.2.0"));
        assert!(is_newer("2.0.0", "1.9.9"));
        assert!(is_newer("1.10.0", "1.2.0"));
        assert!(is_newer("v1.3.0", "1.2.0"));
        assert!(!is_newer("1.2.0", "1.2.0"));
        assert!(!is_newer("1.2.0", "1.2.1"));
        assert!(!is_newer("1.2.0", "1.10.0"));
    }

    #[test]
    fn version_compare_ignores_prerelease_and_build_suffixes() {
        assert!(!is_newer("1.2.0-rc.1", "1.2.0"));
        assert!(!is_newer("1.2.0+build5", "1.2.0"));
        assert!(!is_newer("v1.2.0-rc.1", "1.2.0"));
        // A pre-release of a newer patch is still newer than the old release.
        assert!(is_newer("1.2.1-rc.1", "1.2.0"));
        // Tags are not always plain "vX.Y.Z" — scan to the first digit.
        assert!(is_newer("enclave-1.3.0", "1.2.0"));
        assert_eq!(parse_version("1.2.0-rc.1"), vec![1, 2, 0]);
        assert_eq!(parse_version("1.2.0+build5"), vec![1, 2, 0]);
    }

    #[test]
    fn picks_the_platform_asset_from_release_assets() {
        let mk = |name: &str| GhAsset {
            name: name.into(),
            browser_download_url: format!("https://example.com/{name}"),
            size: 1024,
            digest: Some(format!("sha256:{}", sha256_hex(name.as_bytes()))),
        };
        let assets = vec![
            mk("enclave_1.3.0_amd64.deb"),
            mk("enclave_1.3.0_amd64.AppImage"),
            mk("enclave_1.3.0_x64-setup.exe"),
            mk("enclave_1.3.0_aarch64.dmg"),
            mk("app-universal-release-unsigned.apk"),
            mk("enclave-android-release.apk"),
            mk("source.tar.gz"),
        ];
        let picked = pick_asset(&assets).expect("one platform asset always matches");
        assert!(!picked.name.contains("unsigned"), "unsigned APK must never be picked");
        assert!(matches!(
            picked.name.as_str(),
            "enclave_1.3.0_amd64.AppImage"
                | "enclave_1.3.0_x64-setup.exe"
                | "enclave_1.3.0_aarch64.dmg"
                | "app-universal-release.apk"
        ));
    }

    #[test]
    fn sha256_digest_parsing_is_strict_and_case_insensitive() {
        // Known vector: sha256("abc").
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let hex = sha256_hex(b"enclave");
        assert_eq!(expected_sha256(&format!("sha256:{hex}")).unwrap(), hex);
        assert_eq!(
            expected_sha256(&format!("sha256:{}", hex.to_uppercase())).unwrap(),
            hex
        );
        assert!(expected_sha256("md5:abcd").is_err());
        assert!(expected_sha256("sha256:not-a-hex-digest").is_err());
        assert!(expected_sha256(&format!("sha256:{hex}00")).is_err());
    }
}
