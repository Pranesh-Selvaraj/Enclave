package com.enclave.app

import android.content.Intent
import android.os.Bundle
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge

class MainActivity : TauriActivity() {
  private var webView: RustWebView? = null
  private var pendingAction: String? = null

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    this.webView = webView as? RustWebView
    pendingAction?.let { route(it) }
    pendingAction = null
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    // Read before super.onCreate — onWebViewCreate fires during it and
    // needs the pending action already set. It clears the action when it
    // routes, so the post-super call below is a no-op in that case.
    pendingAction = intent?.getStringExtra("enclave:action")
    super.onCreate(savedInstanceState)
    pendingAction?.let { route(it) }
    pendingAction = null
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
