package com.enclave.app

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.Service
import android.content.Context
import android.content.Intent
import android.net.wifi.WifiManager
import android.os.Build
import android.os.IBinder

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
    val notification: Notification =
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
        Notification.Builder(this, CHANNEL_ID)
          .setContentTitle("Enclave sync active")
          .setContentText("Discovering and syncing with your devices on this network")
          .setSmallIcon(R.drawable.ic_launcher_foreground)
          .setOngoing(true)
          .build()
      } else {
        @Suppress("DEPRECATION")
        Notification.Builder(this)
          .setContentTitle("Enclave sync active")
          .setContentText("Discovering and syncing with your devices on this network")
          .setSmallIcon(R.drawable.ic_launcher_foreground)
          .setOngoing(true)
          .build()
      }
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
    wifiLock?.let { if (it.isHeld) it.release() }
    wifiLock = null
    super.onDestroy()
  }

  companion object {
    private const val CHANNEL_ID = "sync"
    private const val NOTIFICATION_ID = 42
  }
}
