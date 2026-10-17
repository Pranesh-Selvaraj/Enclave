package com.enclave.app.ui

import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.CornerRadius
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import com.enclave.app.core.Core
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/**
 * Create or unlock the vault. Creating one shows the recovery phrase once and
 * only continues after the user acknowledges it — the same contract as the
 * shipping app.
 */
@Composable
fun VaultScreen(onUnlocked: () -> Unit) {
    val scope = rememberCoroutineScope()
    val initialized = remember { Core.ffi.isVaultInitialized() }

    var password by remember { mutableStateOf("") }
    var confirm by remember { mutableStateOf("") }
    var mnemonic by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(horizontal = 28.dp)
                .imePadding(),
            verticalArrangement = Arrangement.Center,
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            KeyholeMark()
            Spacer(Modifier.height(20.dp))
            Text(
                "Enclave",
                style = MaterialTheme.typography.headlineLarge,
                fontWeight = FontWeight.SemiBold,
            )
            Spacer(Modifier.height(6.dp))
            Text(
                "Your knowledge stays yours — no cloud, no accounts, no telemetry.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
            Spacer(Modifier.height(32.dp))

            val words = mnemonic
            when {
                words != null -> RecoveryPhrase(words, onConfirmed = onUnlocked)

                initialized -> {
                    OutlinedTextField(
                        value = password,
                        onValueChange = { password = it; error = null },
                        label = { Text("Password") },
                        singleLine = true,
                        visualTransformation = PasswordVisualTransformation(),
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Spacer(Modifier.height(16.dp))
                    Button(
                        onClick = {
                            scope.launch {
                                busy = true
                                error = null
                                runCatching {
                                    withContext(Dispatchers.IO) { Core.ffi.unlockWithPassword(password) }
                                }
                                    .onSuccess { onUnlocked() }
                                    .onFailure { error = "Could not unlock the vault — check your password." }
                                busy = false
                            }
                        },
                        enabled = !busy && password.isNotEmpty(),
                        modifier = Modifier.fillMaxWidth().height(50.dp),
                    ) { Text(if (busy) "Unlocking…" else "Unlock") }
                }

                else -> {
                    OutlinedTextField(
                        value = password,
                        onValueChange = { password = it; error = null },
                        label = { Text("Choose a password") },
                        singleLine = true,
                        visualTransformation = PasswordVisualTransformation(),
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Spacer(Modifier.height(12.dp))
                    OutlinedTextField(
                        value = confirm,
                        onValueChange = { confirm = it; error = null },
                        label = { Text("Confirm password") },
                        singleLine = true,
                        visualTransformation = PasswordVisualTransformation(),
                        modifier = Modifier.fillMaxWidth(),
                    )
                    Spacer(Modifier.height(16.dp))
                    Button(
                        onClick = {
                            if (password != confirm) {
                                error = "Passwords do not match."
                                return@Button
                            }
                            if (password.isEmpty()) {
                                error = "Choose a password first."
                                return@Button
                            }
                            scope.launch {
                                busy = true
                                error = null
                                runCatching {
                                    withContext(Dispatchers.IO) { Core.ffi.createVault(password) }
                                }
                                    .onSuccess { mnemonic = it }
                                    .onFailure { error = "Could not create the vault: ${it.message}" }
                                busy = false
                            }
                        },
                        enabled = !busy,
                        modifier = Modifier.fillMaxWidth().height(50.dp),
                    ) { Text(if (busy) "Creating…" else "Create vault") }
                    Spacer(Modifier.height(14.dp))
                    Text(
                        "A 12-word recovery phrase is generated next. It is the only way to recover the vault if you forget the password — write it down and keep it offline.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        textAlign = TextAlign.Center,
                    )
                }
            }

            error?.let {
                Spacer(Modifier.height(16.dp))
                Text(it, color = MaterialTheme.colorScheme.error, style = MaterialTheme.typography.bodyMedium)
            }
        }
    }
}

@Composable
private fun RecoveryPhrase(words: String, onConfirmed: () -> Unit) {
    val parts = remember(words) { words.trim().split(Regex("\\s+")) }
    Text("Your recovery phrase", style = MaterialTheme.typography.titleMedium, fontWeight = FontWeight.SemiBold)
    Spacer(Modifier.height(4.dp))
    Text(
        "Write these ${parts.size} words down in order and keep them offline.",
        style = MaterialTheme.typography.bodySmall,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
        textAlign = TextAlign.Center,
    )
    Spacer(Modifier.height(16.dp))
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        parts.withIndex().chunked(3).forEach { row ->
            Row(horizontalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                row.forEach { (index, word) ->
                    Surface(
                        color = MaterialTheme.colorScheme.surfaceVariant,
                        shape = RoundedCornerShape(10.dp),
                        modifier = Modifier.weight(1f),
                    ) {
                        Row(
                            modifier = Modifier.padding(horizontal = 10.dp, vertical = 8.dp),
                            verticalAlignment = Alignment.CenterVertically,
                        ) {
                            Text(
                                "${index + 1}",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            Spacer(Modifier.width(6.dp))
                            Text(word, style = MaterialTheme.typography.bodyMedium, fontWeight = FontWeight.Medium)
                        }
                    }
                }
            }
        }
    }
    Spacer(Modifier.height(20.dp))
    Button(onClick = onConfirmed, modifier = Modifier.fillMaxWidth().height(50.dp)) {
        Text("I've saved my recovery phrase")
    }
}

/** The keyhole-vault mark, drawn so it needs no raster asset. */
@Composable
fun KeyholeMark(size: Dp = 64.dp) {
    val accent = MaterialTheme.colorScheme.primary
    Canvas(Modifier.size(size)) {
        val side = this.size.minDimension
        drawRoundRect(
            color = accent,
            topLeft = Offset.Zero,
            size = Size(side, side),
            cornerRadius = CornerRadius(side * 0.30f),
        )
        val cx = side / 2f
        val cy = side / 2f
        drawCircle(color = Color.White, radius = side * 0.17f, center = Offset(cx, cy - side * 0.07f))
        drawRoundRect(
            color = Color.White,
            topLeft = Offset(cx - side * 0.07f, cy - side * 0.02f),
            size = Size(side * 0.14f, side * 0.26f),
            cornerRadius = CornerRadius(side * 0.07f),
        )
    }
}
