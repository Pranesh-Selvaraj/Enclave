//! UniFFI binding generator (Kotlin). Run with:
//!   cargo run -p core-api --features uniffi,cli --bin uniffi-bindgen -- \
//!     generate --library <libcore_api.so> --language kotlin --out-dir <dir>
fn main() {
    uniffi::uniffi_bindgen_main()
}
