package com.enclave.app.ui

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.AlertDialog
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
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.core.content.FileProvider
import com.enclave.app.BuildConfig
import com.enclave.app.WidgetCacheBridge
import com.enclave.app.core.Core
import com.enclave.app.sync.SyncController
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.core_api.UpdateInfo
import java.io.File

private enum class SettingsSheet { THEME, ACCENT }

/**
 * Settings: appearance (device-local), security (the widget-hide dial is vault
 * data because both the widget renderer and the app must agree on it), data
 * (backup export), updates (opt-in, like the desktop app) and about.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    onBack: () -> Unit,
    onLock: () -> Unit,
    onRestored: (String) -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var sheet by remember { mutableStateOf<SettingsSheet?>(null) }
    var hideWidgets by remember { mutableStateOf<String?>(null) }
    var message by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var pendingBackupPath by remember { mutableStateOf<String?>(null) }

    var updateInfo by remember { mutableStateOf<UpdateInfo?>(null) }
    var updateBusy by remember { mutableStateOf<String?>(null) }
    var updateError by remember { mutableStateOf<String?>(null) }

    var restoreBytes by remember { mutableStateOf<ByteArray?>(null) }
    var restoreSecret by remember { mutableStateOf("") }
    var restoreError by remember { mutableStateOf<String?>(null) }
    var showRestore by remember { mutableStateOf(false) }
    var restoreBusy by remember { mutableStateOf(false) }

    LaunchedEffect(Unit) {
        hideWidgets = withContext(Dispatchers.IO) {
            runCatching { Core.ffi.getSetting("widget_hide_locked") }.getOrNull()
        }
    }

    // The export writes the encrypted snapshot the core produced to wherever
    // the user points the system picker (SAF); no storage permission needed.
    val exportLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.CreateDocument("application/octet-stream"),
    ) { uri ->
        val path = pendingBackupPath
        pendingBackupPath = null
        if (uri != null && path != null) {
            message = copyToUri(context, path, uri)
        }
        // The private snapshot is a duplicate once copied (or abandoned).
        if (path != null) runCatching { File(path).delete() }
    }

    // Restoring is destructive and re-authenticates, so the picked file is
    // only staged here; the confirmation dialog collects the secret.
    val restoreLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            withContext(Dispatchers.IO) { runCatching { readUri(context, uri) } }
                .onSuccess {
                    restoreBytes = it
                    restoreSecret = ""
                    restoreError = null
                    showRestore = true
                }
                .onFailure { message = "Could not read the backup: ${it.message}" }
        }
    }

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = { Text("Settings") },
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
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            item { SectionTitle("Appearance") }
            item {
                SettingsRow("Theme", AppPrefs.themeMode.label) { sheet = SettingsSheet.THEME }
            }
            item {
                SettingsRow(
                    title = "Accent",
                    value = AppPrefs.accent.label,
                    swatch = AppPrefs.accent.light,
                ) { sheet = SettingsSheet.ACCENT }
            }

            item { SectionTitle("Security") }
            item {
                Surface(
                    color = MaterialTheme.colorScheme.surface,
                    shape = RoundedCornerShape(14.dp),
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Row(Modifier.padding(14.dp), verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text(
                                "Hide widgets while locked",
                                style = MaterialTheme.typography.bodyLarge,
                            )
                            Text(
                                "Widgets show a lock placeholder instead of note content.",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        Switch(
                            checked = hideWidgets == "true",
                            enabled = hideWidgets != null,
                            onCheckedChange = { on ->
                                hideWidgets = on.toString()
                                scope.launch {
                                    withContext(Dispatchers.IO) {
                                        runCatching {
                                            Core.ffi.setSetting("widget_hide_locked", on.toString())
                                        }
                                    }
                                    WidgetCacheBridge.requestRefresh()
                                }
                            },
                        )
                    }
                }
            }
            item { SettingsRow("Lock now", "The password is required to open it again") { onLock() } }

            item { SectionTitle("Data") }
            item {
                SettingsRow(
                    title = "Export vault backup",
                    value = if (busy) "Preparing…" else null,
                ) {
                    scope.launch {
                        busy = true
                        message = null
                        val result = withContext(Dispatchers.IO) {
                            runCatching { Core.ffi.backupVault() }
                        }
                        busy = false
                        result
                            .onSuccess { path ->
                                pendingBackupPath = path
                                exportLauncher.launch(File(path).name)
                            }
                            .onFailure { message = "Could not create the backup: ${it.message}" }
                    }
                }
            }
            item {
                Text(
                    "The backup is the encrypted vault database — without your password or recovery phrase it stays unreadable.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            item {
                SettingsRow(
                    title = "Restore vault backup",
                    value = if (restoreBusy) "Restoring…" else null,
                ) {
                    message = null
                    runCatching { restoreLauncher.launch(arrayOf("application/octet-stream", "*/*")) }
                        .onFailure { message = "Could not open the file picker: ${it.message}" }
                }
            }
            item {
                Text(
                    "Restoring replaces this device's vault and locks it. Only a backup of this vault opens with your password or recovery phrase.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            item { SectionTitle("Updates") }
            item {
                SettingsRow(
                    title = "Check for updates",
                    value = updateBusy,
                ) {
                    if (updateBusy != null) return@SettingsRow
                    scope.launch {
                        updateBusy = "Checking…"
                        updateError = null
                        message = null
                        runCatching {
                            withContext(Dispatchers.IO) {
                                Core.ffi.checkUpdate(BuildConfig.VERSION_NAME)
                            }
                        }
                            .onSuccess { info ->
                                updateBusy = null
                                if (info.updateAvailable) {
                                    updateInfo = info
                                } else {
                                    message = "You're on the latest version (${info.currentVersion})."
                                }
                            }
                            .onFailure {
                                updateBusy = null
                                updateError = "Update check failed: ${it.message}"
                            }
                    }
                }
            }
            item {
                Text(
                    "Enclave never checks on its own — only when you ask. A download is verified against the pinned release key before the installer runs.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                (updateError ?: message)?.let {
                    Spacer(Modifier.height(6.dp))
                    Text(
                        it,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                }
            }

            item { SectionTitle("About") }
            item { SettingsRow("Version", BuildConfig.VERSION_NAME, onClick = null) }
            item {
                Text(
                    "Your vault stays on this device. Enclave has no cloud, no accounts and no telemetry.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 4.dp),
                )
            }
        }
    }

    when (sheet) {
        SettingsSheet.THEME -> OptionSheet(
            title = "Theme",
            options = ThemeMode.entries.map { it to it.label },
            selected = AppPrefs.themeMode,
            onSelect = { AppPrefs.setThemeMode(context, it); sheet = null },
            onDismiss = { sheet = null },
        )

        SettingsSheet.ACCENT -> OptionSheet(
            title = "Accent",
            options = Accent.entries.map { it to it.label },
            selected = AppPrefs.accent,
            onSelect = { AppPrefs.setAccent(context, it); sheet = null },
            onDismiss = { sheet = null },
            swatch = { it.light },
        )

        null -> Unit
    }

    updateInfo?.let { info ->
        AlertDialog(
            onDismissRequest = { if (updateBusy == null) updateInfo = null },
            title = { Text("Enclave ${info.latestVersion} is available") },
            text = {
                Column(
                    Modifier.heightIn(max = 320.dp).verticalScroll(rememberScrollState()),
                ) {
                    Text(
                        "You are on ${info.currentVersion}.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(Modifier.height(10.dp))
                    Text(
                        info.notes.ifBlank { "No release notes for this version." },
                        style = MaterialTheme.typography.bodyMedium,
                    )
                    info.assetSize?.let { size ->
                        Spacer(Modifier.height(10.dp))
                        Text(
                            "Download: ${size / 1024uL / 1024uL} MB",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    updateError?.let {
                        Spacer(Modifier.height(10.dp))
                        Text(
                            it,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                    }
                }
            },
            confirmButton = {
                TextButton(
                    enabled = updateBusy == null,
                    onClick = {
                        scope.launch {
                            updateBusy = "Downloading…"
                            updateError = null
                            runCatching {
                                withContext(Dispatchers.IO) {
                                    Core.ffi.downloadUpdate(context.cacheDir.absolutePath)
                                }
                            }
                                .onSuccess { path ->
                                    updateBusy = null
                                    updateInfo = null
                                    val started = runCatching { installApk(context, path) }
                                    message = started.fold(
                                        onSuccess = { "Update verified — starting the installer…" },
                                        onFailure = { "Could not start the installer: ${it.message}" },
                                    )
                                }
                                .onFailure {
                                    updateBusy = null
                                    updateError = "Update failed: ${it.message}"
                                }
                        }
                    },
                ) { Text(updateBusy ?: "Download & install") }
            },
            dismissButton = {
                TextButton(enabled = updateBusy == null, onClick = { updateInfo = null }) {
                    Text("Later")
                }
            },
        )
    }

    if (showRestore && restoreBytes != null) {
        AlertDialog(
            onDismissRequest = {
                if (!restoreBusy) {
                    showRestore = false
                    restoreBytes = null
                }
            },
            title = { Text("Restore vault backup") },
            text = {
                Column {
                    Text(
                        "This replaces this device's vault with the backup and locks it.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    Spacer(Modifier.height(12.dp))
                    OutlinedTextField(
                        value = restoreSecret,
                        onValueChange = { restoreSecret = it; restoreError = null },
                        label = { Text("Password or recovery phrase") },
                        maxLines = 3,
                        modifier = Modifier.fillMaxWidth(),
                    )
                    restoreError?.let {
                        Spacer(Modifier.height(8.dp))
                        Text(
                            it,
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.error,
                        )
                    }
                }
            },
            confirmButton = {
                TextButton(
                    enabled = !restoreBusy && restoreSecret.isNotBlank(),
                    onClick = {
                        val bytes = restoreBytes ?: return@TextButton
                        scope.launch {
                            restoreBusy = true
                            restoreError = null
                            // Background sync must not run against a vault that
                            // is being replaced.
                            SyncController.pause(context)
                            val result = withContext(Dispatchers.IO) {
                                runCatching { Core.ffi.restoreVault(restoreSecret, bytes) }
                            }
                            restoreBusy = false
                            result
                                .onSuccess {
                                    showRestore = false
                                    restoreBytes = null
                                    restoreSecret = ""
                                    WidgetCacheBridge.requestRefresh()
                                    onRestored("Vault restored — unlock with its password or recovery phrase.")
                                }
                                .onFailure { restoreError = "Restore failed: ${it.message}" }
                        }
                    },
                ) { Text(if (restoreBusy) "Restoring…" else "Restore") }
            },
            dismissButton = {
                TextButton(
                    enabled = !restoreBusy,
                    onClick = { showRestore = false; restoreBytes = null },
                ) { Text("Cancel") }
            },
        )
    }
}

@Composable
private fun SectionTitle(text: String) {
    Text(
        text,
        style = MaterialTheme.typography.titleMedium,
        fontWeight = FontWeight.SemiBold,
        modifier = Modifier.padding(top = 8.dp, bottom = 2.dp),
    )
}

@Composable
private fun SettingsRow(
    title: String,
    value: String? = null,
    swatch: Color? = null,
    onClick: (() -> Unit)? = null,
) {
    Surface(
        color = MaterialTheme.colorScheme.surface,
        shape = RoundedCornerShape(14.dp),
        modifier = Modifier.fillMaxWidth().then(
            if (onClick != null) Modifier.clickable(onClick = onClick) else Modifier,
        ),
    ) {
        Row(Modifier.padding(14.dp), verticalAlignment = Alignment.CenterVertically) {
            if (swatch != null) {
                Box(Modifier.size(18.dp).background(swatch, CircleShape))
                Spacer(Modifier.width(12.dp))
            }
            Text(
                title,
                style = MaterialTheme.typography.bodyLarge,
                modifier = Modifier.weight(1f),
            )
            value?.let {
                Text(
                    it,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            if (onClick != null) {
                Icon(
                    Icons.AutoMirrored.Filled.KeyboardArrowRight,
                    contentDescription = null,
                    tint = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

/** Copies the core's private snapshot to the user-chosen SAF location. */
private fun copyToUri(context: Context, path: String, uri: Uri): String = runCatching {
    val bytes = File(path).readBytes()
    context.contentResolver.openOutputStream(uri, "wt")?.use { it.write(bytes) }
        ?: error("the chosen location could not be opened")
    "Backup saved (${bytes.size / 1024} KB)."
}.getOrElse { "Could not save the backup: ${it.message}" }

/** Reads a picked backup, refusing anything absurd before it reaches the core. */
private fun readUri(context: Context, uri: Uri): ByteArray {
    val maxBytes = 512L * 1024 * 1024
    context.contentResolver.openInputStream(uri).use { input ->
        requireNotNull(input) { "the chosen file could not be opened" }
        val out = java.io.ByteArrayOutputStream()
        val buffer = ByteArray(64 * 1024)
        var total = 0L
        while (true) {
            val read = input.read(buffer)
            if (read <= 0) break
            total += read
            require(total <= maxBytes) { "that file is too large to be a vault backup" }
            out.write(buffer, 0, read)
        }
        return out.toByteArray()
    }
}

/**
 * Hands a verified APK to the Android package installer. It over-installs the
 * running app (user data is kept) and needs REQUEST_INSTALL_PACKAGES.
 */
private fun installApk(context: Context, path: String) {
    val uri = FileProvider.getUriForFile(
        context,
        "${context.packageName}.fileprovider",
        File(path),
    )
    val intent = Intent(Intent.ACTION_VIEW).apply {
        setDataAndType(uri, "application/vnd.android.package-archive")
        addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
    }
    context.startActivity(intent)
}
