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
