//! Android-only bridge: start/stop the foreground `SyncService` (Kotlin) that
//! keeps the P2P sync stack alive while the app is backgrounded (issue #60).
//!
//! The service is started when sync is enabled and stopped when it is
//! disabled or the vault locks — no service (and no notification) for users
//! who never enable P2P sync.

#[cfg(target_os = "android")]
pub fn set_service(enable: bool) -> Result<(), String> {
    use jni::objects::{JObject, JValue};
    use jni::JavaVM;

    // tao (via tauri) registers the activity and keeps the AndroidContext —
    // the same source the windowing code uses.
    let ctx = tauri::tao::platform::android::prelude::main_android_context()
        .ok_or("android context not ready (activity not created)")?;
    let vm = unsafe { JavaVM::from_raw(ctx.java_vm.cast()) }.map_err(|e| e.to_string())?;
    // AttachGuard derefs to JNIEnv — call methods through it directly.
    let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;
    let activity = unsafe { JObject::from_raw(ctx.context_jobject.cast()) };

    let intent = env
        .new_object("android/content/Intent", "()V", &[])
        .map_err(|e| e.to_string())?;
    let service_name = env
        .new_string("com.enclave.app.SyncService")
        .map_err(|e| e.to_string())?;
    env.call_method(
        &intent,
        "setClassName",
        "(Landroid/content/Context;Ljava/lang/String;)Landroid/content/Intent;",
        &[JValue::Object(&activity), JValue::Object(&service_name)],
    )
    .map_err(|e| e.to_string())?;

    if enable {
        // startForegroundService (API 26+) — minSdk 24 needs the legacy path
        // below it.
        let sdk_int = env
            .get_static_field("android/os/Build$VERSION", "SDK_INT", "I")
            .and_then(|v| v.i())
            .unwrap_or(0);
        let method = if sdk_int >= 26 { "startForegroundService" } else { "startService" };
        env.call_method(
            &activity,
            method,
            "(Landroid/content/Intent;)Landroid/content/ComponentName;",
            &[JValue::Object(&intent)],
        )
        .map_err(|e| e.to_string())?;
    } else {
        env.call_method(
            &activity,
            "stopService",
            "(Landroid/content/Intent;)Z",
            &[JValue::Object(&intent)],
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(not(target_os = "android"))]
pub fn set_service(_enable: bool) -> Result<(), String> {
    Ok(())
}

/// Tell the Kotlin shell that vault data changed, so it can refresh the
/// home-screen widget cache — replacing the old 20 s poll (issue #127).
/// Best effort: a missing/broken bridge must never fail a save.
#[cfg(target_os = "android")]
pub fn widgets_changed() {
    let _ = with_env(|env| {
        let _ = env.call_static_method(
            "com/enclave/app/WidgetCacheBridge",
            "onCoreChanged",
            "()V",
            &[],
        );
    });
}

#[cfg(not(target_os = "android"))]
pub fn widgets_changed() {}

/// Run `f` on a thread attached to the app's JVM. None when the Android
/// context is not ready yet (e.g. very early startup).
#[cfg(target_os = "android")]
fn with_env<T>(f: impl FnOnce(&mut jni::JNIEnv) -> T) -> Option<T> {
    use jni::JavaVM;

    let ctx = tauri::tao::platform::android::prelude::main_android_context()?;
    let vm = unsafe { JavaVM::from_raw(ctx.java_vm.cast()) }.ok()?;
    let mut env = vm.attach_current_thread().ok()?;
    Some(f(&mut env))
}

/// Read a user-picked SAF document (`content://`). The dialog plugin returns
/// content URIs on Android, not filesystem paths, so plain std::fs cannot
/// touch them (issue #38).
#[cfg(target_os = "android")]
pub fn read_content_uri(uri: &str) -> Result<Vec<u8>, String> {
    use jni::objects::{JByteArray, JValue};

    with_env(|env| -> Result<Vec<u8>, String> {
        let juri = env.new_string(uri).map_err(|e| e.to_string())?;
        let result = env
            .call_static_method(
                "com/enclave/app/AndroidDocuments",
                "readUri",
                "(Ljava/lang/String;)[B",
                &[JValue::Object(&juri)],
            )
            .map_err(|e| e.to_string())?;
        let obj = result.l().map_err(|e| e.to_string())?;
        if obj.is_null() {
            return Err(format!("Could not read the selected file ({uri})"));
        }
        let arr: JByteArray = obj.into();
        env.convert_byte_array(&arr).map_err(|e| e.to_string())
    })
    .unwrap_or_else(|| Err("Android context is not ready — try again".into()))
}

#[cfg(not(target_os = "android"))]
pub fn read_content_uri(uri: &str) -> Result<Vec<u8>, String> {
    Err(format!("content:// URIs are Android-only: {uri}"))
}

/// Write/truncate a SAF document chosen with the create-document picker.
#[cfg(target_os = "android")]
pub fn write_content_uri(uri: &str, data: &[u8]) -> Result<(), String> {
    use jni::objects::JValue;

    with_env(|env| -> Result<(), String> {
        let juri = env.new_string(uri).map_err(|e| e.to_string())?;
        let jdata = env.byte_array_from_slice(data).map_err(|e| e.to_string())?;
        let result = env
            .call_static_method(
                "com/enclave/app/AndroidDocuments",
                "writeUri",
                "(Ljava/lang/String;[B)Z",
                &[JValue::Object(&juri), JValue::Object(&jdata)],
            )
            .map_err(|e| e.to_string())?;
        if result.z().map_err(|e| e.to_string())? {
            Ok(())
        } else {
            Err(format!("Could not write the selected file ({uri})"))
        }
    })
    .unwrap_or_else(|| Err("Android context is not ready — try again".into()))
}

#[cfg(not(target_os = "android"))]
pub fn write_content_uri(uri: &str, _data: &[u8]) -> Result<(), String> {
    Err(format!("content:// URIs are Android-only: {uri}"))
}
