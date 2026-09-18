package com.enclave.app

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.net.wifi.WifiManager
import android.os.Bundle
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import uniffi.core_api.FfiCore

/**
 * The full Enclave app: the desktop UI (Svelte) in the Tauri web view, with
 * all features — editor, whiteboards, tables, graph, search, sync, settings.
 *
 * Android-only surfaces route in here: share sheet → capture, widgets and the
 * Quick Settings tile → capture/open note, launcher shortcuts → capture.
 * While the app is in memory it also keeps the widget cache fresh (widgets
 * render the Keystore-wrapped cache, never the vault).
 */
class MainActivity : TauriActivity() {
  private var webView: RustWebView? = null
  private var pendingRoute: String? = null

  // Held for the app's lifetime: without it Android's Wi-Fi driver filters
  // multicast, so the Rust mDNS daemon (core-network) sees no peers and sync
  // never connects. ponytail: kept while the app is alive regardless of
  // foreground state — battery cost of a multicast lock is negligible and
  // backgrounded sync reconnects need it too.
  private var multicastLock: WifiManager.MulticastLock? = null
  private val widgetScope = CoroutineScope(Dispatchers.IO + SupervisorJob())

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    this.webView = webView as? RustWebView
    pendingRoute?.let { routeTo(it) }
    pendingRoute = null
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    // JNA loads the shared UniFFI surface from the Tauri library.
    System.setProperty("jna.library.path", applicationInfo.nativeLibraryDir)
    System.setProperty("uniffi.component.core_api.libraryOverride", "enclave_lib")
    enableEdgeToEdge()
    // P2P sync: acquire before any network activity — mDNS is broken without it.
    val wifi = applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
    multicastLock = wifi.createMulticastLock("enclave-mdns").apply {
      setReferenceCounted(false)
      acquire()
    }
    // Read before super.onCreate — onWebViewCreate fires during it and needs
    // the pending route already set.
    pendingRoute = routeFor(intent)
    super.onCreate(savedInstanceState)
    pendingRoute?.let { routeTo(it) }
    pendingRoute = null
    startWidgetCacheLoop()
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    routeFor(intent)?.let { routeTo(it) }
  }

  override fun onDestroy() {
    widgetScope.cancel()
    multicastLock?.let { if (it.isHeld) it.release() }
    multicastLock = null
    super.onDestroy()
  }

  /** Maps an incoming Android intent onto an in-app route. */
  private fun routeFor(intent: Intent?): String? {
    if (intent == null) return null
    return when (intent.action) {
      EnclaveIntents.ACTION_OPEN_NOTE ->
        intent.getStringExtra(EnclaveIntents.EXTRA_DOC_ID)?.takeIf { it.isNotBlank() }?.let { "/$it" }

      EnclaveIntents.ACTION_CAPTURE,
      EnclaveIntents.ACTION_NEW_NOTE,
      EnclaveIntents.ACTION_NEW_CHECKLIST ->
        captureRoute(intent.getStringExtra(EnclaveIntents.EXTRA_TEXT))

      Intent.ACTION_SEND -> {
        @Suppress("DEPRECATION")
        captureRoute(intent.getStringExtra(Intent.EXTRA_TEXT))
      }

      else -> null
    }
  }

  private fun captureRoute(text: String?): String {
    val trimmed = text?.trim().orEmpty()
    return if (trimmed.isEmpty()) "/capture" else "/capture?text=" + Uri.encode(trimmed)
  }

  private fun routeTo(path: String) {
    webView?.loadUrlMainThread("tauri://localhost$path")
  }

  /**
   * Keep the widget cache in step while the app is in memory: refresh after
   * unlock/saves and render the lock placeholder when the vault locks.
   */
  private fun startWidgetCacheLoop() {
    widgetScope.launch {
      while (isActive) {
        try {
          val core = FfiCore(applicationInfo.dataDir)
          val unlocked = core.isUnlocked()
          var changed = WidgetStore.vaultUnlocked(applicationContext) != unlocked
          WidgetStore.setVaultUnlocked(applicationContext, unlocked)
          // "Hide widgets while locked" lives in the vault (Settings) so both
          // shells and devices agree; mirror it locally for widget renders.
          val hide = core.getSetting("widget_hide_locked") == "true"
          if (WidgetStore.hideWhenLocked(applicationContext) != hide) {
            WidgetStore.setHideWhenLocked(applicationContext, hide)
            changed = true
          }
          if (unlocked) {
            WidgetStore.refreshAndUpdate(applicationContext, core)
          } else if (changed) {
            WidgetStore.updateAll(applicationContext)
          }
        } catch (_: Exception) {
          // Vault locked or core mid-flight — the next tick recovers.
        }
        delay(20_000)
      }
    }
  }
}
