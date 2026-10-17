//! Self-update over GitHub Releases: check, download and verify.
//!
//! Portable on purpose — the desktop shell and the native Android app both
//! drive it. Trust comes from the pinned Ed25519 release key, not from TLS or
//! the GitHub account: every downloaded asset is checked against GitHub's
//! SHA-256 digest *and* a detached signature over that digest, so a
//! compromised release or proxy cannot install arbitrary code (issue #83).

use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::Path;

use crate::{CoreError, CoreResult};

const REPO: &str = "Pranesh-Selvaraj/Enclave";
const UA: &str = concat!("enclave-core/", env!("CARGO_PKG_VERSION"));

/// Pinned Ed25519 release-signing public key (32 bytes, hex).
///
/// Rotating it requires republishing this constant (and therefore an update
/// signed by the *current* key, or a manual install) plus a new
/// `UPDATE_SIGNING_KEY` CI secret. Keep the private key offline; see
/// `scripts/sign-release.mjs`.
pub const UPDATE_PUBKEY_HEX: &str =
    "966b2a4e0f69852842e9fd2b916724f9e5288fa9c8eea18b235a35a50b593e90";

/// What a shell shows the user after a check.
#[derive(Clone, Debug, serde::Serialize)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub notes: String,
    pub asset_name: Option<String>,
    pub asset_size: Option<u64>,
}

/// The asset selected by the last check. Kept in the core so a download never
/// takes a URL from the caller.
#[derive(Clone, Debug)]
pub struct UpdateAsset {
    name: String,
    url: String,
    size: u64,
    digest: Option<String>,
    /// `browser_download_url` of the `<name>.sig` asset, if the release has one.
    sig_url: Option<String>,
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
    /// GitHub's `sha256:<hex>` asset digest (optional in older API responses).
    digest: Option<String>,
}

/// Decode a hex string (even length, either case). Trims surrounding
/// whitespace so a `.sig` file with a trailing newline verifies.
pub fn decode_hex(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Verify a detached Ed25519 signature over an asset's SHA-256 hex digest.
/// Returns a user-facing error on any malformed input or mismatch (fail
/// closed); never panics.
pub fn verify_update_signature(
    digest_hex: &str,
    sig_hex: &str,
    pubkey_hex: &str,
) -> Result<(), String> {
    use ed25519_dalek::{Signature, VerifyingKey};

    let key_bytes = decode_hex(pubkey_hex).ok_or("malformed update public key")?;
    let key_bytes: [u8; 32] = key_bytes
        .try_into()
        .map_err(|_| "malformed update public key")?;
    let key = VerifyingKey::from_bytes(&key_bytes)
        .map_err(|e| format!("invalid update public key: {e}"))?;
    let sig_bytes = decode_hex(sig_hex).ok_or("malformed update signature")?;
    let sig = Signature::from_slice(&sig_bytes).map_err(|_| "malformed update signature")?;
    key.verify_strict(digest_hex.as_bytes(), &sig)
        .map_err(|_| "Update signature check failed — the download was discarded".to_string())
}

/// `sha256:<hex>` → lowercase hex. Any other scheme is refused (fail closed).
fn expected_sha256(digest: &str) -> CoreResult<String> {
    let hex = digest
        .strip_prefix("sha256:")
        .ok_or_else(|| CoreError::InvalidInput(format!("unsupported update digest scheme: {digest}")))?;
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CoreError::InvalidInput(format!("malformed sha256 digest: {digest}")));
    }
    Ok(hex.to_ascii_lowercase())
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

pub fn is_newer(latest: &str, current: &str) -> bool {
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

/// Query the latest GitHub release for `current_version`. Returns the info to
/// show and the asset the core should download next (never a caller-supplied
/// URL). Blocking — call it off the async runtime's worker threads.
pub fn check_for_update(current_version: &str) -> CoreResult<(UpdateInfo, Option<UpdateAsset>)> {
    let json = ureq::get(&format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("User-Agent", UA)
        .header("Accept", "application/vnd.github+json")
        .call()
        .map_err(|e| CoreError::Network(format!("Update check failed: {e}")))?
        .body_mut()
        .read_to_string()
        .map_err(|e| CoreError::Network(format!("Update check failed: {e}")))?;

    let release: GhRelease = serde_json::from_str(&json)
        .map_err(|e| CoreError::Network(format!("Unexpected release response: {e}")))?;
    let latest = release.tag_name.trim_start_matches('v').to_string();
    let asset = pick_asset(&release.assets);

    let info = UpdateInfo {
        current_version: current_version.to_string(),
        latest_version: latest.clone(),
        update_available: is_newer(&latest, current_version),
        notes: release.body.clone().unwrap_or_default(),
        asset_name: asset.map(|a| a.name.clone()),
        asset_size: asset.map(|a| a.size),
    };

    let pending = asset.map(|a| UpdateAsset {
        name: a.name.clone(),
        url: a.browser_download_url.clone(),
        size: a.size,
        digest: a.digest.clone(),
        sig_url: release
            .assets
            .iter()
            .find(|s| s.name == format!("{}.sig", a.name))
            .map(|s| s.browser_download_url.clone()),
    });

    Ok((info, pending))
}

/// Stream `asset` into `dest_dir`, verifying its SHA-256 digest, size and
/// Ed25519 signature. Any failure deletes the file and aborts (issue #83).
pub fn download_update(asset: &UpdateAsset, dest_dir: &Path) -> CoreResult<String> {
    let expected = asset.digest.as_deref().map(expected_sha256).transpose()?;
    std::fs::create_dir_all(dest_dir).map_err(|e| CoreError::Io(e.to_string()))?;
    let dest = dest_dir.join(sanitize_filename(&asset.name));

    let resp = ureq::get(&asset.url)
        .header("User-Agent", UA)
        .call()
        .map_err(|e| CoreError::Network(format!("Download failed: {e}")))?;
    let mut body = resp.into_body();
    let mut reader = body.as_reader();
    let mut out = std::fs::File::create(&dest).map_err(|e| CoreError::Io(e.to_string()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut received = 0u64;

    loop {
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                let _ = std::fs::remove_file(&dest);
                return Err(CoreError::Network(format!("Download failed: {e}")));
            }
        };
        if let Err(e) = out.write_all(&buf[..n]) {
            let _ = std::fs::remove_file(&dest);
            return Err(CoreError::Io(e.to_string()));
        }
        hasher.update(&buf[..n]);
        received += n as u64;
    }
    drop(out);

    let actual: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if let Some(expected) = &expected {
        if !actual.eq_ignore_ascii_case(expected) {
            let _ = std::fs::remove_file(&dest);
            return Err(CoreError::InvalidInput(
                "Update failed its integrity check (sha256 mismatch) — the download was discarded".into(),
            ));
        }
    }
    // A short/truncated body must not be installed even when no digest is
    // advertised; GitHub always reports the asset size.
    if asset.size > 0 && received != asset.size {
        let _ = std::fs::remove_file(&dest);
        return Err(CoreError::InvalidInput(format!(
            "Update size mismatch ({received} bytes received, {} expected) — the download was discarded",
            asset.size
        )));
    }
    // Signature last: it authenticates the bytes whose digest was just
    // computed, so a tampered download is rejected even if GitHub's own digest
    // field was tampered with too. Fail closed when unsigned.
    let Some(sig_url) = &asset.sig_url else {
        let _ = std::fs::remove_file(&dest);
        return Err(CoreError::InvalidInput(
            "This release has no update signature — refusing to install an unverified download".into(),
        ));
    };
    let sig_hex = ureq::get(sig_url)
        .header("User-Agent", UA)
        .call()
        .map_err(|e| CoreError::Network(format!("Signature download failed: {e}")))?
        .body_mut()
        .read_to_string()
        .map_err(|e| CoreError::Network(format!("Signature download failed: {e}")))?;
    if let Err(e) = verify_update_signature(&actual, &sig_hex, UPDATE_PUBKEY_HEX) {
        let _ = std::fs::remove_file(&dest);
        return Err(CoreError::InvalidInput(e));
    }

    Ok(dest.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    /// sha256("abc") — a digest whose value is fixed and public, so the tests
    /// need no hash dependency.
    const DIGEST: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    fn keypair() -> (String, String) {
        let signing = SigningKey::from_bytes(&[7u8; 32]);
        let pub_hex: String = signing
            .verifying_key()
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let sig_hex: String = signing
            .sign(DIGEST.as_bytes())
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        (pub_hex, sig_hex)
    }

    #[test]
    fn accepts_a_signature_over_the_exact_digest() {
        let (pub_hex, sig_hex) = keypair();
        assert!(verify_update_signature(DIGEST, &sig_hex, &pub_hex).is_ok());
        // A `.sig` file with a trailing newline must still verify.
        assert!(verify_update_signature(DIGEST, &format!("{sig_hex}\n"), &pub_hex).is_ok());
    }

    #[test]
    fn rejects_a_different_digest_or_key() {
        let (pub_hex, sig_hex) = keypair();
        let other_digest = "0000000000000000000000000000000000000000000000000000000000000000";
        assert!(verify_update_signature(other_digest, &sig_hex, &pub_hex).is_err());

        let other_pub: String = SigningKey::from_bytes(&[9u8; 32])
            .verifying_key()
            .to_bytes()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert!(verify_update_signature(DIGEST, &sig_hex, &other_pub).is_err());
    }

    #[test]
    fn malformed_inputs_fail_closed() {
        let (pub_hex, sig_hex) = keypair();
        assert!(verify_update_signature(DIGEST, "not-hex", &pub_hex).is_err());
        assert!(verify_update_signature(DIGEST, "", &pub_hex).is_err());
        assert!(verify_update_signature(DIGEST, &sig_hex, "not-hex").is_err());
        assert!(verify_update_signature(DIGEST, &sig_hex, "").is_err());
        // 31-byte key (odd valid prefix) is not a VerifyingKey length.
        assert!(verify_update_signature(DIGEST, &sig_hex, &pub_hex[..62]).is_err());
    }

    #[test]
    fn pinned_update_key_is_a_valid_ed25519_point() {
        let bytes = decode_hex(UPDATE_PUBKEY_HEX).expect("pinned public key is hex");
        assert_eq!(bytes.len(), 32, "ed25519 public keys are 32 bytes");
        let key: [u8; 32] = bytes.try_into().unwrap();
        ed25519_dalek::VerifyingKey::from_bytes(&key).expect("pinned public key is a valid point");
    }

    #[test]
    fn sha_digest_parsing_is_strict_and_case_insensitive() {
        // Known vector: sha256("abc").
        let hex = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(expected_sha256(&format!("sha256:{hex}")).unwrap(), hex);
        // The scheme is exact; only the hex is case-insensitive.
        assert_eq!(
            expected_sha256(&format!("sha256:{}", hex.to_uppercase())).unwrap(),
            hex
        );
        assert!(expected_sha256("md5:abcd").is_err());
        assert!(expected_sha256("SHA256:abcd").is_err());
        assert!(expected_sha256("sha256:not-a-hex-digest").is_err());
        assert!(expected_sha256(&format!("sha256:{hex}00")).is_err());
    }

    #[test]
    fn version_comparison_is_numeric_and_ignores_suffixes() {
        assert!(is_newer("1.10.0", "1.2.0"));
        assert!(!is_newer("1.2.0", "1.10.0"));
        assert!(!is_newer("1.13.0", "1.13.0"));
        assert!(!is_newer("1.13.0-rc.1", "1.13.0"));
        assert!(is_newer("1.13.1", "1.13.0"));
    }

    #[test]
    fn asset_pick_skips_unsigned_and_matches_the_platform() {
        let suffix = platform_suffixes()[0];
        let assets = vec![
            GhAsset {
                name: format!("enclave-unsigned{suffix}"),
                browser_download_url: "https://example.invalid/unsigned".into(),
                size: 1,
                digest: None,
            },
            GhAsset {
                name: format!("enclave-1.0.0{suffix}"),
                browser_download_url: "https://example.invalid/signed".into(),
                size: 2,
                digest: None,
            },
        ];
        let picked = pick_asset(&assets).expect("a signed asset exists");
        assert_eq!(picked.browser_download_url, "https://example.invalid/signed");
    }
}
