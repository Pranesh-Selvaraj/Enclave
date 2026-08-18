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
use std::io::{Read, Write};
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
}

/// "1.2.0" / "v1.2.0" → [1, 2, 0]. Non-numeric segments (prerelease suffixes
/// like "-rc.1") are skipped — GitHub's "latest" already excludes
/// prereleases, so a skipped suffix can only make versions look equal, and
/// equal never triggers an update.
fn parse_version(v: &str) -> Vec<u32> {
    v.trim_start_matches('v')
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
pub async fn check_for_update() -> Result<UpdateInfo, String> {
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

/// Streams the release asset into the app cache dir; emits `update-progress`
/// events ({received, total, percent}) and returns the local file path.
#[tauri::command]
pub async fn download_update(
    app: tauri::AppHandle,
    url: String,
    filename: String,
) -> Result<String, String> {
    let cache_dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    let dest = cache_dir.join(sanitize_filename(&filename));
    let dest_clone = dest.clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn_blocking(move || {
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
        let mut buf = [0u8; 64 * 1024];
        let mut received = 0u64;
        let mut last_pct = u32::MAX;
        loop {
            let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n]).map_err(|e| e.to_string())?;
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
        Ok::<String, String>(dest_clone.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Launches the downloaded installer/APK. Over-installs — no uninstall, user
/// data is kept on every platform.
#[tauri::command]
pub async fn install_update(app: tauri::AppHandle, path: String) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        let state = app.state::<UpdaterState>();
        state
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
    fn picks_the_platform_asset_from_release_assets() {
        let mk = |name: &str| GhAsset {
            name: name.into(),
            browser_download_url: format!("https://example.com/{name}"),
            size: 1024,
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
}
