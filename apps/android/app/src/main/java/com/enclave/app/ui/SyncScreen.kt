package com.enclave.app.ui

import android.content.ClipData
import android.content.ClipboardManager
import android.content.Context
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.TopAppBarDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.enclave.app.core.Core
import com.enclave.app.sync.SyncController
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.core_api.NetworkStatus
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

private val ConnectedGreen = Color(0xFF2E7D32)

/**
 * P2P sync: on/off, this device's address, the peers on the network and the
 * manual add-a-peer path for networks that block mDNS. Status is polled while
 * the screen is open; everything else is core state.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SyncScreen(onBack: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    val enabled = SyncController.enabled
    var status by remember { mutableStateOf<NetworkStatus?>(null) }
    var host by remember { mutableStateOf("") }
    var port by remember { mutableStateOf("") }
    var error by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        SyncController.refresh(context)
        while (true) {
            status = withContext(Dispatchers.IO) {
                runCatching { Core.ffi.networkStatus() }.getOrNull()
            }
            delay(2500)
        }
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text("Sync") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.background,
                ),
            )
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding),
            contentPadding = PaddingValues(20.dp),
            verticalArrangement = Arrangement.spacedBy(14.dp),
        ) {
            item {
                Surface(
                    color = MaterialTheme.colorScheme.surface,
                    shape = RoundedCornerShape(16.dp),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text(
                                if (enabled) "Sync is on" else "Sync is off",
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.SemiBold,
                            )
                            Text(
                                if (enabled) {
                                    "Devices on this Wi-Fi discover each other and sync directly."
                                } else {
                                    "Nothing leaves this device until you turn sync on."
                                },
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        Switch(
                            checked = enabled,
                            onCheckedChange = { on ->
                                if (on) SyncController.enable(context) else SyncController.disable(context)
                            },
                        )
                    }
                }
            }

            item {
                val current = status
                Surface(
                    color = MaterialTheme.colorScheme.surface,
                    shape = RoundedCornerShape(16.dp),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Column(Modifier.padding(16.dp), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        Text(
                            "This device",
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        val address = when {
                            current == null -> "…"
                            current.localHost.isBlank() -> "finding an address…"
                            else -> "${current.localHost}:${current.port}"
                        }
                        Row(verticalAlignment = Alignment.CenterVertically) {
                            Text(
                                address,
                                style = MaterialTheme.typography.titleMedium,
                                fontFamily = FontFamily.Monospace,
                                modifier = Modifier.weight(1f),
                            )
                            TextButton(
                                onClick = { copyToClipboard(context, address) },
                                enabled = current?.localHost?.isNotBlank() == true,
                            ) { Text("Copy") }
                        }
                        Text(
                            current?.lastSyncAt?.let { "Last synced ${formatEpoch(it)}" }
                                ?: "Not synced yet",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }

            item {
                Text(
                    "Devices",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.padding(top = 4.dp),
                )
            }

            val peers = status?.peers.orEmpty()
            if (peers.isEmpty()) {
                item {
                    Text(
                        if (enabled) "No other devices found yet." else "Turn sync on to look for your devices.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            } else {
                items(peers, key = { it.id }) { peer ->
                    Surface(
                        color = MaterialTheme.colorScheme.surface,
                        shape = RoundedCornerShape(14.dp),
                        modifier = Modifier.fillMaxWidth(),
                    ) {
                        Row(Modifier.padding(14.dp), verticalAlignment = Alignment.CenterVertically) {
                            Box(
                                Modifier
                                    .size(9.dp)
                                    .background(
                                        if (peer.connected) ConnectedGreen
                                        else MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.35f),
                                        CircleShape,
                                    ),
                            )
                            Spacer(Modifier.width(10.dp))
                            Column(Modifier.weight(1f)) {
                                Text(
                                    peer.name.ifBlank { peer.id.take(8) },
                                    style = MaterialTheme.typography.bodyLarge,
                                )
                                Text(
                                    "${peer.host}:${peer.port}",
                                    style = MaterialTheme.typography.labelSmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                            Text(
                                if (peer.connected) "Connected" else "Found",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }

            item {
                Text(
                    "Add a device by address",
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    modifier = Modifier.padding(top = 4.dp),
                )
                Text(
                    "For networks that block mDNS discovery (some routers, guest Wi-Fi, VPNs).",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 2.dp, bottom = 10.dp),
                )
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    OutlinedTextField(
                        value = host,
                        onValueChange = { host = it; error = null },
                        label = { Text("host") },
                        singleLine = true,
                        modifier = Modifier.weight(1.5f),
                    )
                    OutlinedTextField(
                        value = port,
                        onValueChange = { port = it.filter(Char::isDigit); error = null },
                        label = { Text("port") },
                        singleLine = true,
                        modifier = Modifier.weight(1f),
                    )
                }
                Spacer(Modifier.size(10.dp))
                Button(
                    onClick = {
                        val parsedPort = port.toUShortOrNull()
                        if (host.isBlank() || parsedPort == null) {
                            error = "Enter a host and a port."
                            return@Button
                        }
                        scope.launch {
                            runCatching {
                                withContext(Dispatchers.IO) {
                                    Core.ffi.connectPeer(host.trim(), parsedPort)
                                }
                            }
                                .onSuccess {
                                    error = null
                                    host = ""
                                    port = ""
                                }
                                .onFailure { error = "Could not connect: ${it.message}" }
                        }
                    },
                    enabled = enabled,
                ) { Text("Connect") }
                error?.let {
                    Text(
                        it,
                        color = MaterialTheme.colorScheme.error,
                        style = MaterialTheme.typography.bodySmall,
                        modifier = Modifier.padding(top = 8.dp),
                    )
                }
            }
        }
    }
}

private fun copyToClipboard(context: Context, text: String) {
    val clipboard = context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
    clipboard.setPrimaryClip(ClipData.newPlainText("Enclave sync address", text))
}

private fun formatEpoch(millis: ULong): String =
    SimpleDateFormat("d MMM, HH:mm", Locale.getDefault()).format(Date(millis.toLong()))
