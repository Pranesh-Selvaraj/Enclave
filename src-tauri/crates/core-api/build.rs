//! Windows link requirements for the `core-api` cdylib.
//!
//! Desktops link the system OpenSSL (on Windows: the vcpkg static build). The
//! main binary gets user32/crypt32 transitively through Tauri/WebView2, but
//! this cdylib is linked on its own and must name the Windows API libraries
//! the OpenSSL static libs reference (CertOpenStore, GetProcessWindowStation,
//! …).
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rustc-link-lib=crypt32");
        println!("cargo:rustc-link-lib=user32");
    }
}
