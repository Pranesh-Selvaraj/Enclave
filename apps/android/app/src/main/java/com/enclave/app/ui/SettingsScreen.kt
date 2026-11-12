package com.enclave.app.ui

import android.content.Context
import android.content.Intent
import android.net.Uri
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
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
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.core.content.FileProvider
import com.enclave.app.BuildConfig
import com.enclave.app.R
import com.enclave.app.WidgetCacheBridge
import com.enclave.app.core.Core
import com.enclave.app.sync.SyncController
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.core_api.UpdateInfo
import java.io.File
import java.util.Locale

private enum class SettingsSheet { THEME, ACCENT, AUTO_LOCK, TEXT_SIZE, PAGE_WIDTH }

/**
 * Settings: appearance (device-local), security (the widget-hide dial is vault
 * data because both the widget renderer and the app must agree on it), data
 * (backup export), updates (opt-in, like the desktop app) and about.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    onBack: () -> Unit,
    onRestored: (String) -> Unit,
) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var sheet by remember { mutableStateOf<SettingsSheet?>(null) }
    var hideWidgets by remember { mutableStateOf<String?>(null) }
    var message by remember { mutableStateOf<String?>(null) }
    var backupMessage by remember { mutableStateOf<String?>(null) }
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
    var showCustomAccent by remember { mutableStateOf(false) }
    var customHex by remember { mutableStateOf("") }

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
            backupMessage = copyToUri(context, path, uri)
        } else if (path != null) {
            backupMessage = "Backup cancelled."
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
            withContext(Dispatchers.IO) { runCatching { readBackupBytes(context, uri) } }
                .onSuccess {
                    restoreBytes = it
                    restoreSecret = ""
                    restoreError = null
                    showRestore = true
                }
                .onFailure { message = "Could not read the backup: ${it.message}" }
        }
    }

    val dark = MaterialTheme.colorScheme.background.luminance() < 0.5f
    val lockup = painterResource(
        if (dark) R.drawable.ic_enclave_lockup_paper else R.drawable.ic_enclave_lockup_ink,
    )

    Scaffold(
        containerColor = MaterialTheme.colorScheme.background,
        topBar = {
            TopAppBar(
                title = {},
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
            item {
                Column(
                    modifier = Modifier.fillMaxWidth().padding(top = 8.dp, bottom = 6.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) {
                    Image(
                        painter = lockup,
                        contentDescription = "Enclave",
                        contentScale = ContentScale.Fit,
                        modifier = Modifier.width(170.dp).aspectRatio(347.1f / 48f),
                    )
                    Spacer(Modifier.height(6.dp))
                    Text(
                        "Settings",
                        style = MaterialTheme.typography.headlineSmall,
                        fontWeight = FontWeight.SemiBold,
                    )
                }
            }

            item { SectionTitle("Appearance") }
            item {
                SettingsRow("Theme", AppPrefs.themeMode.label) { sheet = SettingsSheet.THEME }
            }
            if (AppPrefs.themeMode != ThemeMode.MONOCHROME) {
                item {
                    SettingsRow(
                        title = "Accent",
                        value = AppPrefs.accent.label,
                        swatch = AppPrefs.accentLight,
                    ) { sheet = SettingsSheet.ACCENT }
                }
            }
            item {
                SettingsRow("Editor text size", AppPrefs.editorTextSize.label) {
                    sheet = SettingsSheet.TEXT_SIZE
                }
            }
            item {
                SettingsRow("Page width", AppPrefs.editorWidth.label) {
                    sheet = SettingsSheet.PAGE_WIDTH
                }
            }

            item { SectionTitle("Security") }
            item {
                SettingsRow("Auto-lock", AppPrefs.autoLock.label) {
                    sheet = SettingsSheet.AUTO_LOCK
                }
            }
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
                                "Widgets show a lock placeholder.",
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

            item { SectionTitle("Data") }
            item {
                SettingsRow(
                    title = "Export vault backup",
                    value = if (busy) "Preparing…" else null,
                ) {
                    scope.launch {
                        busy = true
                        backupMessage = null
                        val result = withContext(Dispatchers.IO) {
                            runCatching { Core.ffi.backupVault() }
                        }
                        busy = false
                        result
                            .onSuccess { path ->
                                pendingBackupPath = path
                                exportLauncher.launch(File(path).name)
                            }
                            .onFailure { backupMessage = "Could not create the backup: ${it.message}" }
                    }
                }
            }
            item {
                Text(
                    "Encrypted; unreadable without your password.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                // The result belongs next to the action: it used to be printed
                // in the Updates section far below, so a finished backup looked
                // like nothing had happened.
                backupMessage?.let {
                    Spacer(Modifier.height(6.dp))
                    Text(
                        it,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.primary,
                    )
                }
            }
            item {
                SettingsRow(
                    title = "Restore vault backup",
                    value = if (restoreBusy) "Restoring…" else null,
                ) {
                    backupMessage = null
                    runCatching { restoreLauncher.launch(arrayOf("application/octet-stream", "*/*")) }
                        .onFailure { backupMessage = "Could not open the file picker: ${it.message}" }
                }
            }
            item {
                Text(
                    "Replaces this vault and locks it.",
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
                    "Only when you ask; downloads are signature-verified.",
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
                    "Type: Inter · JetBrains Mono (OFL).",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(top = 4.dp),
                )
            }
            item {
                Text(
                    "Local-only. No cloud, accounts or telemetry.",
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
            onSelect = {
                if (it == Accent.CUSTOM) {
                    customHex = (AppPrefs.customAccent ?: AppPrefs.accentLight).toHex()
                    showCustomAccent = true
                } else {
                    AppPrefs.setAccent(context, it)
                }
                sheet = null
            },
            onDismiss = { sheet = null },
            swatch = { if (it == Accent.CUSTOM) AppPrefs.customAccent ?: it.light else it.light },
        )

        SettingsSheet.AUTO_LOCK -> OptionSheet(
            title = "Auto-lock",
            options = AutoLock.entries.map { it to it.label },
            selected = AppPrefs.autoLock,
            onSelect = { AppPrefs.setAutoLock(context, it); sheet = null },
            onDismiss = { sheet = null },
        )

        SettingsSheet.TEXT_SIZE -> OptionSheet(
            title = "Editor text size",
            options = EditorTextSize.entries.map { it to it.label },
            selected = AppPrefs.editorTextSize,
            onSelect = { AppPrefs.setEditorTextSize(context, it); sheet = null },
            onDismiss = { sheet = null },
        )

        SettingsSheet.PAGE_WIDTH -> OptionSheet(
            title = "Page width",
            options = EditorWidth.entries.map { it to it.label },
            selected = AppPrefs.editorWidth,
            onSelect = { AppPrefs.setEditorWidth(context, it); sheet = null },
            onDismiss = { sheet = null },
        )

        null -> Unit
    }

    if (showCustomAccent) {
        CustomAccentDialog(
            initialHex = customHex,
            onDismiss = { showCustomAccent = false },
            onSave = {
                AppPrefs.setCustomAccent(context, it)
                showCustomAccent = false
            },
        )
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
private fun CustomAccentDialog(
    initialHex: String,
    onDismiss: () -> Unit,
    onSave: (Color) -> Unit,
) {
    val initial = parseHexColor(initialHex) ?: Color(0xFF6C5CE7)
    val hsv = remember {
        FloatArray(3).also { android.graphics.Color.colorToHSV(initial.toArgb(), it) }
    }
    var hue by remember { mutableStateOf(hsv[0]) }
    var saturation by remember { mutableStateOf(hsv[1]) }
    var value by remember { mutableStateOf(hsv[2].coerceAtLeast(0.05f)) }
    // Select the current value so typing replaces it instead of appending.
    var field by remember {
        mutableStateOf(TextFieldValue(initialHex, TextRange(0, initialHex.length)))
    }
    val color = Color.hsv(hue, saturation, value)
    val valid = parseHexColor(field.text) != null

    fun applyState(next: Color) {
        val channels = FloatArray(3)
        android.graphics.Color.colorToHSV(next.toArgb(), channels)
        hue = channels[0]
        saturation = channels[1]
        value = channels[2]
        val hex = next.toHex()
        field = TextFieldValue(hex, TextRange(0, hex.length))
    }

    fun syncField() {
        val hex = Color.hsv(hue, saturation, value).toHex()
        field = TextFieldValue(hex, TextRange(0, hex.length))
    }

    val quick = listOf(
        "#6C5CE7", "#4C6EF5", "#0F766E", "#B45309",
        "#BE123C", "#2E7D32", "#C2410C", "#0E7490",
    )

    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Custom accent") },
        text = {
            Column {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(
                        Modifier
                            .size(44.dp)
                            .background(color, CircleShape),
                    )
                    Spacer(Modifier.width(14.dp))
                    OutlinedTextField(
                        value = field,
                        onValueChange = {
                            field = it
                            parseHexColor(it.text)?.let(::applyState)
                        },
                        label = { Text("Hex color") },
                        singleLine = true,
                        isError = !valid,
                        modifier = Modifier.weight(1f),
                    )
                }

                Spacer(Modifier.height(12.dp))
                // Saturation / brightness square for the current hue.
                Canvas(
                    Modifier
                        .fillMaxWidth()
                        .height(150.dp)
                        .clip(RoundedCornerShape(12.dp))
                        .pointerInput(hue) {
                            awaitEachGesture {
                                fun apply(position: Offset) {
                                    saturation = (position.x / size.width.toFloat()).coerceIn(0f, 1f)
                                    value = (1f - position.y / size.height.toFloat()).coerceIn(0f, 1f)
                                    syncField()
                                }
                                val down = awaitFirstDown()
                                apply(down.position)
                                down.consume()
                                while (true) {
                                    val event = awaitPointerEvent()
                                    val change = event.changes.firstOrNull { it.pressed } ?: break
                                    apply(change.position)
                                    change.consume()
                                }
                            }
                        },
                ) {
                    drawRect(Brush.horizontalGradient(listOf(Color.White, Color.hsv(hue, 1f, 1f))))
                    drawRect(Brush.verticalGradient(listOf(Color.Transparent, Color.Black)))
                    drawCircle(
                        color = Color.White,
                        radius = 9.dp.toPx(),
                        center = Offset(saturation * size.width, (1f - value) * size.height),
                        style = Stroke(width = 2.dp.toPx()),
                    )
                }

                Spacer(Modifier.height(10.dp))
                // Hue bar.
                Canvas(
                    Modifier
                        .fillMaxWidth()
                        .height(26.dp)
                        .clip(CircleShape)
                        .pointerInput(Unit) {
                            awaitEachGesture {
                                fun apply(position: Offset) {
                                    hue = (position.x / size.width.toFloat() * 360f).coerceIn(0f, 360f)
                                    syncField()
                                }
                                val down = awaitFirstDown()
                                apply(down.position)
                                down.consume()
                                while (true) {
                                    val event = awaitPointerEvent()
                                    val change = event.changes.firstOrNull { it.pressed } ?: break
                                    apply(change.position)
                                    change.consume()
                                }
                            }
                        },
                ) {
                    drawRect(
                        Brush.horizontalGradient(
                            listOf(
                                Color.Red, Color.Yellow, Color.Green,
                                Color.Cyan, Color.Blue, Color.Magenta, Color.Red,
                            ),
                        ),
                    )
                    drawCircle(
                        color = Color.White,
                        radius = 10.dp.toPx(),
                        center = Offset(hue / 360f * size.width, size.height / 2),
                        style = Stroke(width = 2.dp.toPx()),
                    )
                }

                Spacer(Modifier.height(12.dp))
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    quick.forEach { hexValue ->
                        val swatch = parseHexColor(hexValue) ?: return@forEach
                        Box(
                            Modifier
                                .size(24.dp)
                                .background(swatch, CircleShape)
                                .clickable { applyState(swatch) },
                        )
                    }
                }
                if (!valid) {
                    Spacer(Modifier.height(8.dp))
                    Text(
                        "Use a six-digit hex value, like #6C5CE7.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.error,
                    )
                }
            }
        },
        confirmButton = {
            TextButton(enabled = valid, onClick = { parseHexColor(field.text)?.let(onSave) }) {
                Text("Use color")
            }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

private fun parseHexColor(value: String): Color? {
    val clean = value.trim().removePrefix("#")
    if (!Regex("^[0-9A-Fa-f]{6}$").matches(clean)) return null
    return Color(android.graphics.Color.parseColor("#$clean"))
}

private fun Color.toHex(): String = String.format(Locale.US, "#%06X", 0xFFFFFF and toArgb())

@Composable
private fun SectionTitle(text: String) {
    Text(
        text.uppercase(),
        style = MaterialTheme.typography.labelMedium,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = 0.6.sp,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        modifier = Modifier.padding(top = 12.dp, bottom = 2.dp),
    )
}

@Composable
private fun SettingsRow(
    title: String,
    value: String? = null,
    subtitle: String? = null,
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
            Column(Modifier.weight(1f)) {
                Text(
                    title,
                    style = MaterialTheme.typography.bodyLarge,
                )
                subtitle?.let {
                    Text(
                        it,
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
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
