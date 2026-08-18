package com.enclave.app

import android.app.Activity
import android.content.Intent
import androidx.core.content.FileProvider
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.io.File

@InvokeArg
class InstallApkArgs {
  lateinit var path: String
}

/**
 * In-app APK update: hands the downloaded APK to the Android package
 * installer. Over-installs — user data is kept, no uninstall needed.
 * Registered from Rust via `api.register_android_plugin("com.enclave.app",
 * "UpdaterPlugin")` (see src-tauri/src/updater.rs).
 */
@TauriPlugin
class UpdaterPlugin(private val activity: Activity) : Plugin(activity) {

  @Command
  fun installApk(invoke: Invoke) {
    try {
      val args = invoke.parseArgs(InstallApkArgs::class.java)
      val apk = File(args.path)
      val uri = FileProvider.getUriForFile(activity, "${activity.packageName}.fileprovider", apk)
      val intent = Intent(Intent.ACTION_VIEW)
      intent.setDataAndType(uri, "application/vnd.android.package-archive")
      intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
      intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
      activity.startActivity(intent)
      invoke.resolve(JSObject())
    } catch (e: Exception) {
      invoke.reject(e.message ?: "Failed to start the package installer")
    }
  }
}
