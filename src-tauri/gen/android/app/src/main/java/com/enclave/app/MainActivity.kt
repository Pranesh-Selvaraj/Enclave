package com.enclave.app

import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
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
  // Tauri's generated activity disables WebView history navigation
  // (`handleBackNavigation = false`), which left the Android back gesture
  // doing nothing on doc pages and inside overlays. Re-enable it so back
  // pops overlays first and then walks the SPA history — the same history the
  // web shell pushes for drawers/sheets/palette.
  override val handleBackNavigation: Boolean = true

  private var webView: RustWebView? = null
  private var pendingRoute: String? = null

  private val widgetScope = CoroutineScope(Dispatchers.IO + SupervisorJob())

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    this.webView = webView as? RustWebView
    // Android WebView reports env(safe-area-inset-*) as 0 even while the
    // activity draws edge-to-edge. Publish the real system-bar insets (in CSS
    // px) so the shell keeps the top bar clear of the status bar and its
    // controls stay tappable.
    ViewCompat.setOnApplyWindowInsetsListener(webView) { view, insets ->
      val bars = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout()
      )
      val density = view.resources.displayMetrics.density
      val top = (bars.top / density).toInt()
      val bottom = (bars.bottom / density).toInt()
      webView.evaluateJavascript(
        "document.documentElement.style.setProperty('--safe-top','${top}px');" +
          "document.documentElement.style.setProperty('--safe-bottom','${bottom}px');",
        null,
      )
      insets
    }
    ViewCompat.requestApplyInsets(webView)
    // The first dispatch can land before the document exists; re-apply once
    // the initial page has loaded so the CSS vars are never lost.
    webView.postDelayed({ ViewCompat.requestApplyInsets(webView) }, 1500)
    applyPendingRoute()
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    // JNA loads the shared UniFFI surface from the Tauri library.
    System.setProperty("jna.library.path", applicationInfo.nativeLibraryDir)
    System.setProperty("uniffi.component.core_api.libraryOverride", "enclave_lib")
    enableEdgeToEdge()
    // Read before super.onCreate — onWebViewCreate fires during it and needs
    // the pending route already set.
    pendingRoute = routeFor(intent)
    super.onCreate(savedInstanceState)
    applyPendingRoute()
    // Widgets refresh on core writes through the JNI bridge; this only wires
    // the bridge and starts the slow fallback behind it (issue #127).
    WidgetCacheBridge.install(applicationContext)
    // SAF document read/write bridge for import/export (issue #38).
    AndroidDocuments.install(applicationContext)
    startWidgetCacheFallback()
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    routeFor(intent)?.let { routeTo(it) }
  }

  override fun onDestroy() {
    widgetScope.cancel()
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
    val wv = webView ?: return
    // Warm route: navigate inside the SPA through the bridge the web app
    // installs. A full page load would mark the old document hidden, and the
    // Android auto-lock would then wipe the open vault out from under us.
    wv.evaluateJavascript(
      "window.__enclaveRoute && window.__enclaveRoute(${org.json.JSONObject.quote(path)})",
      null,
    )
  }

  /** Cold start: load the route URL once the web view has an origin. */
  private fun loadRouteUrl(path: String) {
    val wv = webView ?: return
    // Tauri serves the app from `http://tauri.localhost` on Android (and
    // Windows) — `tauri://localhost` exists on desktop WebKit only and fails
    // here with ERR_UNKNOWN_URL_SCHEME. Dev builds resolve the same way.
    val base = wv.url?.let { runCatching { java.net.URI(it) }.getOrNull() }
    val target = if (base?.authority != null) {
      java.net.URI(
        base.scheme,
        base.authority,
        path.substringBefore('?'),
        if ('?' in path) path.substringAfter('?') else null,
        null,
      ).toString()
    } else {
      "http://tauri.localhost$path"
    }
    wv.loadUrlMainThread(target)
  }

  /** Applies a cold-start route once the web view has an origin to resolve it. */
  private fun applyPendingRoute() {
    val route = pendingRoute ?: return
    val wv = webView
    if (wv == null || wv.url == null) {
      wv?.postDelayed({ applyPendingRoute() }, 50)
      return
    }
    pendingRoute = null
    loadRouteUrl(route)
  }

  /**
   * Slow safety net behind the core→widget bridge: the bridge refreshes on
   * vault writes/locks (issue #127), this two-minute tick only covers changes
   * that emitted no event, replacing the old 20 s poll.
   */
  private fun startWidgetCacheFallback() {
    widgetScope.launch {
      while (isActive) {
        delay(120_000)
        try {
          WidgetStore.syncFromCore(applicationContext, FfiCore(applicationInfo.dataDir))
        } catch (t: Throwable) {
          android.util.Log.w("EnclaveWidgets", "widget fallback skipped: " + t.message)
        }
      }
    }
  }
}
