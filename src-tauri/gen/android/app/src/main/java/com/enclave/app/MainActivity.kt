package com.enclave.app

import android.content.Context
import android.content.Intent
import android.net.wifi.WifiManager
import android.os.Bundle
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  private var webView: RustWebView? = null
  private var pendingAction: String? = null
  // Held for the app's lifetime: without it Android's Wi-Fi driver filters
  // multicast, so the Rust mDNS daemon (core-network) sees no peers and sync
  // never connects. ponytail: kept while the app is alive regardless of
  // foreground state — battery cost of a multicast lock is negligible and
  // backgrounded sync reconnects need it too.
  private var multicastLock: WifiManager.MulticastLock? = null

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    this.webView = webView as? RustWebView
    pendingAction?.let { route(it) }
    pendingAction = null
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    // P2P sync: acquire before any network activity — mDNS is broken without it.
    val wifi = applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
    multicastLock = wifi.createMulticastLock("enclave-mdns").apply {
      setReferenceCounted(false)
      acquire()
    }
    // Read before super.onCreate — onWebViewCreate fires during it and
    // needs the pending action already set. It clears the action when it
    // routes, so the post-super call below is a no-op in that case.
    pendingAction = intent?.getStringExtra("enclave:action")
    super.onCreate(savedInstanceState)
    pendingAction?.let { route(it) }
    pendingAction = null
  }

  override fun onDestroy() {
    multicastLock?.let { if (it.isHeld) it.release() }
    multicastLock = null
    super.onDestroy()
  }

  override fun onNewIntent(intent: Intent) {
    super.onNewIntent(intent)
    // Widget "New note" → capture screen (singleTask relaunch path).
    pendingAction = intent.getStringExtra("enclave:action")
    pendingAction?.let { route(it) }
    pendingAction = null
  }

  private fun route(action: String) {
    val wv = webView ?: return
    when (action) {
      "capture" -> wv.loadUrlMainThread("tauri://localhost/capture")
    }
  }
}
