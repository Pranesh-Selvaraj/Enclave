package com.enclave.app.ui

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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.KeyboardArrowRight
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
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
import com.enclave.app.BuildConfig
import com.enclave.app.WidgetCacheBridge
import com.enclave.app.core.Core
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

private enum class SettingsSheet { THEME, ACCENT }

/**
 * Settings: appearance (device-local), security (the widget-hide dial is vault
 * data because both the widget renderer and the app must agree on it), and
 * about. Backup and updates land with the SAF/updater work.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(onBack: () -> Unit, onLock: () -> Unit) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    var sheet by remember { mutableStateOf<SettingsSheet?>(null) }
    var hideWidgets by remember { mutableStateOf<String?>(null) }

    LaunchedEffect(Unit) {
        hideWidgets = withContext(Dispatchers.IO) {
            runCatching { Core.ffi.getSetting("widget_hide_locked") }.getOrNull()
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
