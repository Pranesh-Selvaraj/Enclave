package com.enclave.app

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.net.wifi.WifiManager
import android.os.Build
import android.os.IBinder
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.core_api.FfiCore

/**
 * Foreground service that keeps the P2P sync stack alive while the app is
 * backgrounded or the screen is off (issue #60): Android freezes background
 * processes, which kills the Rust runtime's sockets and mDNS. Started by the
 * Rust core via JNI when sync is enabled; stopped when sync is disabled or
 * the vault locks.
 *
 * ponytail: started only while sync is ON — no service (and no
 * notification) when the user never enabled P2P. Upgrade path if needed:
 * network-switched rebinds (connectivity callbacks) live here too.
 */
class SyncService : Service() {

  private var wifiLock: WifiManager.WifiLock? = null
  private val scope = CoroutineScope(Dispatchers.IO + SupervisorJob())

  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    val notify = getSystemService(Context.NOTIFICATION_SERVICE) as NotificationManager
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      notify.createNotificationChannel(
        NotificationChannel(CHANNEL_ID, "P2P sync", NotificationManager.IMPORTANCE_LOW).apply {
          description = "Shown while Enclave syncs with your devices on the local network"
        }
      )
    }
    val notification = buildNotification("Discovering and syncing with your devices on this network")
    // dataSync type is required from API 34 (declared in the manifest);
    // the type constant exists from API 29.
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
      startForeground(
        NOTIFICATION_ID,
        notification,
        android.content.pm.ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC,
      )
    } else {
      startForeground(NOTIFICATION_ID, notification)
    }

    // Live peer status: the notification is the only sync surface while the
    // app is backgrounded, so it reports peers / vault state.
    scope.launch {
      while (isActive) {
        val text = try {
          val core = FfiCore(applicationInfo.dataDir)
          if (!core.isUnlocked()) {
            "Vault locked — unlock Enclave to sync"
          } else {
            val status = core.networkStatus()
            val connected = status.peers.count { it.connected }
            when {
              connected > 0 -> "$connected device(s) connected — syncing"
              status.peers.isNotEmpty() -> "${status.peers.size} device(s) found — connecting"
              else -> "Looking for your devices on this network"
            }
          }
        } catch (_: Exception) {
          "Discovering and syncing with your devices on this network"
        }
        withContext(Dispatchers.Main) {
          runCatching { notify.notify(NOTIFICATION_ID, buildNotification(text)) }
        }
        delay(10_000)
      }
    }

    // Keep Wi-Fi awake while syncing (high-perf mode: multicast + unicast
    // stay alive when the screen is off). Requires ACCESS_WIFI_STATE.
    val wifi = applicationContext.getSystemService(Context.WIFI_SERVICE) as WifiManager
    wifiLock = wifi.createWifiLock(WifiManager.WIFI_MODE_FULL_HIGH_PERF, "enclave-sync").apply {
      setReferenceCounted(false)
      acquire()
    }
    return START_STICKY
  }

  override fun onDestroy() {
    scope.cancel()
    wifiLock?.let { if (it.isHeld) it.release() }
    wifiLock = null
    super.onDestroy()
  }

  /** Notification with a tap-target back into the app. */
  private fun buildNotification(text: String): Notification {
    val open = PendingIntent.getActivity(
      this,
      0,
      Intent(this, KeepActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP),
      PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
    )
    return if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      Notification.Builder(this, CHANNEL_ID)
        .setContentTitle("Enclave sync active")
        .setContentText(text)
        .setSmallIcon(R.drawable.ic_launcher_foreground)
        .setContentIntent(open)
        .setOngoing(true)
        .build()
    } else {
      @Suppress("DEPRECATION")
      Notification.Builder(this)
        .setContentTitle("Enclave sync active")
        .setContentText(text)
        .setSmallIcon(R.drawable.ic_launcher_foreground)
        .setContentIntent(open)
        .setOngoing(true)
        .build()
    }
  }

  companion object {
    private const val CHANNEL_ID = "sync"
    private const val NOTIFICATION_ID = 42
  }
}

/**
 * Start/stop the foreground sync service from the native shell. The Tauri
 * command layer does this from Rust (android_sync.rs); the Kotlin shell talks
 * to the core directly, so it owns the service lifecycle for its own calls.
 */
internal object SyncServiceCtl {
  fun start(context: Context) {
    val intent = Intent(context, SyncService::class.java)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
      context.startForegroundService(intent)
    } else {
      context.startService(intent)
    }
  }

  fun stop(context: Context) {
    context.stopService(Intent(context, SyncService::class.java))
  }
}
