package com.enclave.app.ui

import androidx.compose.animation.AnimatedVisibility
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.Crossfade
import androidx.compose.animation.core.FastOutSlowInEasing
import androidx.compose.animation.core.animateDpAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
import androidx.compose.animation.expandVertically
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.shrinkVertically
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.navigationBars
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawingPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
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
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.graphics.painter.Painter
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.enclave.app.R
import com.enclave.app.core.Core
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

private const val LOCKUP_RATIO = 347.1f / 48f
private val KEYPAD_HEIGHT = 310.dp

/** The core validates the phrase; this is only used to choose the post-restore step. */
private fun looksLikeMnemonic(secret: String): Boolean =
    secret.trim().split(Regex("\\s+")).size >= 12

private enum class PinStage { ENTER, CONFIRM, CONFIRMED }

/**
 * The branded gate: the official lockup lands first, then the minimal vault
 * form fades in. PIN is the Android default and uses an in-app keypad that
 * slides up; the matching password mode uses the system keyboard with an
 * animated IME inset so the form stays fully visible.
 */
@Composable
fun VaultScreen(notice: String? = null, onUnlocked: () -> Unit) {
    val scope = rememberCoroutineScope()
    val context = LocalContext.current
    val keyboard = LocalSoftwareKeyboardController.current
    val focus = LocalFocusManager.current

    var initialized by remember { mutableStateOf<Boolean?>(null) }
    var showForm by remember { mutableStateOf(false) }
    var mode by remember { mutableStateOf(AppPrefs.authMode) }

    var pin by remember { mutableStateOf("") }
    var firstPin by remember { mutableStateOf("") }
    var stage by remember { mutableStateOf(PinStage.ENTER) }
    var keypadOpen by remember { mutableStateOf(false) }

    var password by remember { mutableStateOf("") }
    var confirmPassword by remember { mutableStateOf("") }

    var error by remember { mutableStateOf<String?>(null) }
    var busy by remember { mutableStateOf(false) }
    var mnemonic by remember { mutableStateOf<String?>(null) }

    var restoreBytes by remember { mutableStateOf<ByteArray?>(null) }
    var restoreSecret by remember { mutableStateOf("") }
    var restoreBusy by remember { mutableStateOf(false) }
    var restoreError by remember { mutableStateOf<String?>(null) }
    var restoreNotice by remember { mutableStateOf<String?>(null) }
    var pendingMnemonic by remember { mutableStateOf<String?>(null) }
    var pendingUnlockAfterPin by remember { mutableStateOf(false) }
    // Milliseconds left on the attempt cooldown; ticks down while the gate shows.
    var blockedFor by remember { mutableStateOf(UnlockThrottle.remaining(context)) }
    LaunchedEffect(blockedFor > 0) {
        while (blockedFor > 0) {
            delay(500)
            blockedFor = UnlockThrottle.remaining(context)
        }
    }
    var showForgot by remember { mutableStateOf(false) }
    var forgotPhrase by remember { mutableStateOf("") }
    var forgotBusy by remember { mutableStateOf(false) }
    var forgotError by remember { mutableStateOf<String?>(null) }
    var newPin by remember { mutableStateOf("") }
    var confirmNewPin by remember { mutableStateOf("") }
    var pinSetupBusy by remember { mutableStateOf(false) }
    var pinSetupError by remember { mutableStateOf<String?>(null) }

    val restoreLauncher = rememberLauncherForActivityResult(
        ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri == null) return@rememberLauncherForActivityResult
        scope.launch {
            runCatching { withContext(Dispatchers.IO) { readBackupBytes(context, uri) } }
                .onSuccess {
                    restoreBytes = it
                    restoreSecret = ""
                    restoreError = null
                }
                .onFailure { restoreNotice = "Could not read the backup: ${it.message}" }
        }
    }

    LaunchedEffect(Unit) {
        initialized = withContext(Dispatchers.IO) { Core.ffi.isVaultInitialized() }
        // Let the mark have its moment before the form glides in.
        delay(900)
        showForm = true
    }

    val dark = MaterialTheme.colorScheme.background.luminance() < 0.5f
    val lockup: Painter = painterResource(
        if (dark) R.drawable.ic_enclave_lockup_paper else R.drawable.ic_enclave_lockup_ink,
    )

    fun onDigit(digit: Char) {
        if (busy || mode != AuthMode.PIN || stage == PinStage.CONFIRMED) return
        error = null
        pin = PinRules.append(pin, digit)
    }

    fun onBackspace() {
        if (busy || pin.isEmpty()) return
        error = null
        pin = pin.dropLast(1)
    }

    fun openKeypad() {
        if (busy || mode != AuthMode.PIN) return
        if (stage == PinStage.CONFIRMED) {
            pin = ""
            firstPin = ""
            stage = PinStage.ENTER
        }
        keypadOpen = true
    }

    fun switchMode(next: AuthMode) {
        if (busy || next == mode) return
        mode = next
        AppPrefs.setAuthMode(context, next)
        pin = ""
        firstPin = ""
        stage = PinStage.ENTER
        keypadOpen = false
        password = ""
        confirmPassword = ""
        error = null
        focus.clearFocus()
        keyboard?.hide()
    }

    fun createWithPin() {
        if (busy || stage != PinStage.CONFIRMED || !PinRules.isValid(firstPin)) return
        scope.launch {
            busy = true
            error = null
            runCatching {
                withContext(Dispatchers.IO) { Core.ffi.createVault(firstPin) }
            }
                .onSuccess {
                    mnemonic = it
                    pin = ""
                    firstPin = ""
                }
                .onFailure { error = "Could not create the vault: ${it.message}" }
            busy = false
        }
    }

    fun createWithPassword() {
        when {
            password.isEmpty() -> error = "Choose a password first."
            password != confirmPassword -> error = "Passwords do not match."
            else -> scope.launch {
                busy = true
                error = null
                runCatching {
                    withContext(Dispatchers.IO) { Core.ffi.createVault(password) }
                }
                    .onSuccess { mnemonic = it }
                    .onFailure { error = "Could not create the vault: ${it.message}" }
                busy = false
            }
        }
    }

    fun unlock() {
        if (busy) return
        if (mode == AuthMode.PIN && !PinRules.isValid(pin)) return
        if (mode == AuthMode.PASSWORD && password.isEmpty()) return
        if (UnlockThrottle.remaining(context) > 0) return
        val secret = if (mode == AuthMode.PIN) pin else password
        scope.launch {
            busy = true
            error = null
            runCatching {
                withContext(Dispatchers.IO) { Core.ffi.unlockWithPassword(secret) }
            }
                .onSuccess {
                    UnlockThrottle.reset(context)
                    onUnlocked()
                }
                .onFailure {
                    // Bounded in time, not just in cost: a one-character secret
                    // has ten candidates, so attempts must be throttled.
                    UnlockThrottle.recordFailure(context)
                    blockedFor = UnlockThrottle.remaining(context)
                    error = if (mode == AuthMode.PIN) {
                        "Could not unlock the vault — check your PIN."
                    } else {
                        "Could not unlock the vault — check your password."
                    }
                    if (mode == AuthMode.PIN) pin = ""
                }
            busy = false
        }
    }

    /**
     * Commit the entered PIN. A PIN may be 4–12 digits, so the gate cannot
     * submit on a keystroke count the way it did when four was the only
     * length: the keypad's confirm key and the primary button call this.
     */
    fun onConfirmPin() {
        if (busy || mode != AuthMode.PIN) return
        if (!PinRules.isValid(pin)) {
            error = "Enter your PIN first."
            return
        }
        when {
            initialized == true -> unlock()
            stage == PinStage.ENTER -> {
                firstPin = pin
                pin = ""
                stage = PinStage.CONFIRM
                error = null
            }
            stage == PinStage.CONFIRM -> {
                if (pin == firstPin) {
                    stage = PinStage.CONFIRMED
                    keypadOpen = false
                    error = null
                } else {
                    pin = ""
                    firstPin = ""
                    stage = PinStage.ENTER
                    error = "PINs do not match. Try again."
                }
            }
            else -> Unit
        }
    }

    Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
        Crossfade(
            targetState = mnemonic != null,
            animationSpec = tween(durationMillis = 350),
            label = "vault-content",
        ) { hasWords ->
            if (hasWords) {
                RecoveryPhrase(mnemonic!!, onConfirmed = onUnlocked)
            } else {
                VaultGate(
                    initialized = initialized == true,
                    formVisible = showForm && initialized != null,
                    notice = restoreNotice ?: notice,
                    mode = mode,
                    onMode = ::switchMode,
                    pin = pin,
                    stage = stage,
                    keypadOpen = keypadOpen && mode == AuthMode.PIN,
                    password = password,
                    confirmPassword = confirmPassword,
                    onPassword = { password = it; error = null },
                    onConfirmPassword = { confirmPassword = it; error = null },
                    error = error,
                    busy = busy,
                    blockedFor = blockedFor,
                    lockup = lockup,
                    onDigit = ::onDigit,
                    onBackspace = ::onBackspace,
                    onConfirm = ::onConfirmPin,
                    onOpenKeypad = ::openKeypad,
                    onRestore = {
                        restoreLauncher.launch(arrayOf("application/octet-stream", "*/*"))
                    },
                    onForgot = {
                        forgotError = null
                        showForgot = true
                    },
                    onAction = {
                        when {
                            initialized == true -> unlock()
                            // A PIN may be 4–12 digits, so the button continues
                            // and confirms before it creates.
                            mode == AuthMode.PIN && stage != PinStage.CONFIRMED -> onConfirmPin()
                            mode == AuthMode.PIN -> createWithPin()
                            else -> createWithPassword()
                        }
                    },
                )
            }
        }

        val backup = restoreBytes
        if (backup != null) {
            AlertDialog(
                onDismissRequest = {
                    if (!restoreBusy) {
                        restoreBytes = null
                        restoreSecret = ""
                        restoreError = null
                    }
                },
                title = { Text("Restore a backup") },
                text = {
                    Column {
                        Text(
                            "This replaces this device's vault and locks it.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        if (initialized != true) {
                            Spacer(Modifier.height(8.dp))
                            Text(
                                "On a new install, use the 12-word recovery phrase. The password can only be checked on a device that already holds this vault's key.",
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
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
                            val secret = restoreSecret.trim()
                            scope.launch {
                                restoreBusy = true
                                restoreError = null
                                runCatching {
                                    withContext(Dispatchers.IO) {
                                        Core.ffi.restoreVault(secret, bytes)
                                    }
                                }
                                    .onSuccess {
                                        restoreBusy = false
                                        restoreBytes = null
                                        restoreSecret = ""
                                        pendingUnlockAfterPin = false
                                        if (looksLikeMnemonic(secret)) {
                                            // A phrase restore ends without a usable
                                            // vault.key; set a new PIN to finish.
                                            pendingMnemonic = secret
                                            newPin = ""
                                            confirmNewPin = ""
                                            pinSetupError = null
                                        } else {
                                            initialized = true
                                            mode = if (PinRules.isValid(secret)) {
                                                AuthMode.PIN
                                            } else {
                                                AuthMode.PASSWORD
                                            }
                                            AppPrefs.setAuthMode(context, mode)
                                            restoreNotice = "Vault restored — unlock with its password."
                                        }
                                    }
                                    .onFailure {
                                        restoreBusy = false
                                        restoreError = "Restore failed: ${it.message}" +
                                            if (initialized != true) {
                                                " On a new install, use the 12-word recovery phrase."
                                            } else {
                                                ""
                                            }
                                    }
                            }
                        },
                    ) { Text(if (restoreBusy) "Restoring…" else "Restore") }
                },
                dismissButton = {
                    TextButton(
                        enabled = !restoreBusy,
                        onClick = {
                            restoreBytes = null
                            restoreSecret = ""
                            restoreError = null
                        },
                    ) { Text("Cancel") }
                },
            )
        }

        val mnemonicToProtect = pendingMnemonic
        if (mnemonicToProtect != null) {
            AlertDialog(
                // The restored vault has no usable key file yet; setting the PIN
                // is the only way to finish, so this dialog is not dismissable.
                onDismissRequest = {},
                title = { Text(if (pendingUnlockAfterPin) "Set a new PIN" else "Set a PIN for this device") },
                text = {
                    Column {
                        Text(
                            if (pendingUnlockAfterPin) {
                                "Unlocked with your recovery phrase. Choose a new PIN (4–12 digits) for this device."
                            } else {
                                "The vault is restored. Choose a PIN (4–12 digits) so this device can open it."
                            },
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Spacer(Modifier.height(12.dp))
                        OutlinedTextField(
                            value = newPin,
                            onValueChange = {
                                newPin = it
                                pinSetupError = null
                            },
                            label = { Text("New PIN or password") },
                            singleLine = true,
                            visualTransformation = PasswordVisualTransformation(),
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Text),
                            modifier = Modifier.fillMaxWidth(),
                        )
                        Spacer(Modifier.height(8.dp))
                        OutlinedTextField(
                            value = confirmNewPin,
                            onValueChange = {
                                confirmNewPin = it
                                pinSetupError = null
                            },
                            label = { Text("Confirm it") },
                            singleLine = true,
                            visualTransformation = PasswordVisualTransformation(),
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Text),
                            modifier = Modifier.fillMaxWidth(),
                        )
                        pinSetupError?.let {
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
                        enabled = !pinSetupBusy && PinRules.isValid(newPin) && newPin == confirmNewPin,
                        onClick = {
                            scope.launch {
                                pinSetupBusy = true
                                pinSetupError = null
                                runCatching {
                                    withContext(Dispatchers.IO) {
                                        Core.ffi.setVaultPassword(mnemonicToProtect, newPin)
                                    }
                                }
                                    .onSuccess {
                                        pinSetupBusy = false
                                        pendingMnemonic = null
                                        newPin = ""
                                        confirmNewPin = ""
                                        if (pendingUnlockAfterPin) {
                                            pendingUnlockAfterPin = false
                                            onUnlocked()
                                        } else {
                                            initialized = true
                                            mode = AuthMode.PIN
                                            AppPrefs.setAuthMode(context, AuthMode.PIN)
                                            restoreNotice = "Vault restored — unlock with your new PIN."
                                        }
                                    }
                                    .onFailure {
                                        pinSetupBusy = false
                                        pinSetupError = "Could not set the PIN: ${it.message}"
                                    }
                            }
                        },
                    ) { Text(if (pinSetupBusy) "Saving…" else "Set PIN") }
                },
            )
        }

        if (showForgot) {
            AlertDialog(
                onDismissRequest = {
                    if (!forgotBusy) {
                        showForgot = false
                        forgotPhrase = ""
                        forgotError = null
                    }
                },
                title = { Text("Use your recovery phrase") },
                text = {
                    Column {
                        Text(
                            "Enter the 12 words you saved when this vault was created.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Spacer(Modifier.height(12.dp))
                        OutlinedTextField(
                            value = forgotPhrase,
                            onValueChange = {
                                forgotPhrase = it
                                forgotError = null
                            },
                            label = { Text("12-word recovery phrase") },
                            maxLines = 3,
                            modifier = Modifier.fillMaxWidth(),
                        )
                        forgotError?.let {
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
                        enabled = !forgotBusy && forgotPhrase.isNotBlank(),
                        onClick = {
                            val phrase = forgotPhrase.trim()
                            scope.launch {
                                forgotBusy = true
                                forgotError = null
                                runCatching {
                                    withContext(Dispatchers.IO) { Core.ffi.unlockWithMnemonic(phrase) }
                                }
                                    .onSuccess {
                                        forgotBusy = false
                                        showForgot = false
                                        forgotPhrase = ""
                                        // The phrase unlocked the vault, but the old
                                        // PIN is unknown: replace it before going in.
                                        pendingUnlockAfterPin = true
                                        pendingMnemonic = phrase
                                        newPin = ""
                                        confirmNewPin = ""
                                        pinSetupError = null
                                    }
                                    .onFailure {
                                        forgotBusy = false
                                        forgotError = "That recovery phrase did not unlock the vault."
                                    }
                            }
                        },
                    ) { Text(if (forgotBusy) "Unlocking…" else "Unlock") }
                },
                dismissButton = {
                    TextButton(
                        enabled = !forgotBusy,
                        onClick = {
                            showForgot = false
                            forgotPhrase = ""
                            forgotError = null
                        },
                    ) { Text("Cancel") }
                },
            )
        }
    }
}

@Composable
private fun VaultGate(
    initialized: Boolean,
    formVisible: Boolean,
    notice: String?,
    mode: AuthMode,
    onMode: (AuthMode) -> Unit,
    pin: String,
    stage: PinStage,
    keypadOpen: Boolean,
    password: String,
    confirmPassword: String,
    onPassword: (String) -> Unit,
    onConfirmPassword: (String) -> Unit,
    error: String?,
    busy: Boolean,
    blockedFor: Long,
    lockup: Painter,
    onDigit: (Char) -> Unit,
    onBackspace: () -> Unit,
    onConfirm: () -> Unit,
    onOpenKeypad: () -> Unit,
    onRestore: () -> Unit,
    onForgot: () -> Unit,
    onAction: () -> Unit,
) {
    val density = LocalDensity.current
    val imePx = WindowInsets.ime.getBottom(density)
    val navPx = WindowInsets.navigationBars.getBottom(density)
    val imeExtraPx = (imePx - navPx).coerceAtLeast(0)
    val imeExtra by animateDpAsState(
        targetValue = with(density) { imeExtraPx.toDp() },
        animationSpec = tween(durationMillis = 240),
        label = "ime",
    )
    val imeVisible = imePx > 0

    var appeared by remember { mutableStateOf(false) }
    LaunchedEffect(Unit) { appeared = true }
    val appear by animateFloatAsState(
        targetValue = if (appeared) 1f else 0f,
        animationSpec = tween(durationMillis = 600),
        label = "appear",
    )
    val formProgress by animateFloatAsState(
        targetValue = if (formVisible) 1f else 0f,
        animationSpec = tween(durationMillis = 650),
        label = "form-progress",
    )
    val logoWidth by animateDpAsState(
        targetValue = if (formVisible) 190.dp else 300.dp,
        animationSpec = tween(durationMillis = 650),
        label = "logo-width",
    )
    val primary = MaterialTheme.colorScheme.primary
    // The selected accent (Settings → Accent) drives the glow, the dots, the
    // primary button and the switch pill, so the whole gate follows it.
    val glowAlpha = 0.22f * (1f - formProgress)

    val keypadHeight by animateDpAsState(
        targetValue = if (keypadOpen) KEYPAD_HEIGHT else 0.dp,
        animationSpec = tween(durationMillis = 300),
        label = "keypad",
    )

    val isRegister = !initialized
    val pinFilled = pin.length
    val pinSlots = PinRules.slots(pinFilled)
    val pinReady = PinRules.isValid(pin)
    val passwordReady = password.isNotEmpty() && (!isRegister || password == confirmPassword)

    val pinLabel = when {
        !isRegister -> "Enter your PIN"
        stage == PinStage.ENTER -> "Create a PIN"
        stage == PinStage.CONFIRM -> "Confirm your PIN"
        else -> "PIN confirmed"
    }
    val note = if (isRegister) {
        "Any PIN or password works, with no minimum length — but a longer one is much harder to guess. " +
            "A 12-word recovery phrase is generated next; it is the only way to recover the vault if you forget this, so write it down and keep it offline."
    } else {
        null
    }

    val actionEnabled = when {
        busy || blockedFor > 0 -> false
        initialized && mode == AuthMode.PIN -> pinReady
        initialized -> passwordReady
        // Registering a PIN: the same button continues, confirms and creates.
        mode == AuthMode.PIN -> pinReady
        else -> passwordReady
    }
    val actionLabel = when {
        blockedFor > 0 -> "Try again in " + UnlockThrottle.describe(blockedFor)
        busy && initialized -> "Unlocking…"
        busy -> "Creating…"
        !initialized && mode == AuthMode.PIN && stage == PinStage.ENTER -> "Continue"
        !initialized && mode == AuthMode.PIN && stage == PinStage.CONFIRM -> "Confirm PIN"
        initialized -> "Unlock"
        else -> "Create vault"
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .statusBarsPadding()
            .navigationBarsPadding()
            .padding(bottom = imeExtra),
    ) {
        Box(Modifier.weight(1f).fillMaxWidth()) {
            Column(
                modifier = Modifier
                    .align(Alignment.Center)
                    .fillMaxWidth()
                    .padding(horizontal = 28.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                Box(
                    modifier = Modifier
                        .width(logoWidth)
                        .aspectRatio(LOCKUP_RATIO)
                        // Keep the glow outside the image's render layer: a
                        // layer clips its drawBehind to the logo's bounds while
                        // it fades, which showed up as a box before the circle.
                        .drawBehind {
                            val alpha = glowAlpha * appear
                            if (alpha > 0.001f) {
                                val radius = size.maxDimension * 0.9f
                                drawCircle(
                                    brush = Brush.radialGradient(
                                        0f to primary.copy(alpha = alpha),
                                        0.55f to primary.copy(alpha = alpha * 0.4f),
                                        1f to Color.Transparent,
                                        center = center,
                                        radius = radius,
                                    ),
                                    radius = radius,
                                    center = center,
                                )
                            }
                        },
                    contentAlignment = Alignment.Center,
                ) {
                    Image(
                        painter = lockup,
                        contentDescription = "Enclave",
                        contentScale = ContentScale.Fit,
                        modifier = Modifier
                            .fillMaxSize()
                            .graphicsLayer {
                                alpha = appear
                                scaleX = 0.94f + 0.06f * appear
                                scaleY = 0.94f + 0.06f * appear
                            },
                    )
                }
                AnimatedVisibility(
                    visible = formVisible,
                    enter = fadeIn(tween(420, delayMillis = 80)) +
                        expandVertically(
                            animationSpec = tween(560, easing = FastOutSlowInEasing),
                            expandFrom = Alignment.Top,
                        ),
                    exit = fadeOut(tween(150)) +
                        shrinkVertically(
                            animationSpec = tween(300),
                            shrinkTowards = Alignment.Top,
                        ),
                ) {
                    Column(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        notice?.let {
                            Spacer(Modifier.height(14.dp))
                            Text(
                                it,
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.primary,
                                textAlign = TextAlign.Center,
                            )
                        }
                Spacer(Modifier.height(26.dp))
                AuthSwitch(mode, onMode)
                Spacer(Modifier.height(26.dp))

                if (mode == AuthMode.PIN) {
                    val interaction = remember { MutableInteractionSource() }
                    Column(
                        modifier = Modifier
                            .clickable(
                                interactionSource = interaction,
                                indication = null,
                                onClick = onOpenKeypad,
                            )
                            .padding(vertical = 4.dp),
                        horizontalAlignment = Alignment.CenterHorizontally,
                    ) {
                        Text(
                            pinLabel,
                            style = MaterialTheme.typography.titleSmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Spacer(Modifier.height(16.dp))
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            // Decorative, and deliberately silent: TalkBack
                            // speaks aloud, so announcing "5 characters
                            // entered" would tell anyone in the room the length
                            // and undo the four-dot decoy. The field keeps its
                            // own label ("Enter your PIN"), which is all a
                            // screen-reader user needs to find it.
                            modifier = Modifier.clearAndSetSemantics { },
                        ) {
                            PinDots(filled = pinFilled.coerceAtMost(PinRules.MAX_DOTS), slots = pinSlots, error = error != null)
                            if (pinFilled > PinRules.MAX_DOTS) {
                                Text(
                                    "  ${pinFilled}",
                                    style = MaterialTheme.typography.labelMedium,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                        error?.let {
                            Spacer(Modifier.height(12.dp))
                            Text(
                                it,
                                color = MaterialTheme.colorScheme.error,
                                style = MaterialTheme.typography.bodySmall,
                                textAlign = TextAlign.Center,
                            )
                        }
                    }
                } else {
                    PasswordField(
                        label = if (isRegister) "Choose a password" else "Password",
                        value = password,
                        onValueChange = onPassword,
                    )
                    if (isRegister) {
                        Spacer(Modifier.height(12.dp))
                        PasswordField(
                            label = "Confirm password",
                            value = confirmPassword,
                            onValueChange = onConfirmPassword,
                        )
                    }
                    error?.let {
                        Spacer(Modifier.height(12.dp))
                        Text(
                            it,
                            color = MaterialTheme.colorScheme.error,
                            style = MaterialTheme.typography.bodySmall,
                            textAlign = TextAlign.Center,
                        )
                    }
                }

                        Spacer(Modifier.height(24.dp))
                        Button(
                            onClick = onAction,
                            enabled = actionEnabled,
                            modifier = Modifier.fillMaxWidth().height(50.dp),
                        ) { Text(actionLabel) }
                        Spacer(Modifier.height(6.dp))
                        TextButton(onClick = onRestore, enabled = !busy) {
                            Text(
                                "Restore a backup",
                                style = MaterialTheme.typography.labelLarge,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        if (initialized) {
                            TextButton(onClick = onForgot, enabled = !busy) {
                                Text(
                                    "Forgot PIN or password?",
                                    style = MaterialTheme.typography.labelLarge,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                }
            }

            Column(
                modifier = Modifier.align(Alignment.BottomCenter),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                AnimatedVisibility(
                    visible = note != null && formVisible && !keypadOpen && !imeVisible,
                    enter = fadeIn(tween(300)),
                    exit = fadeOut(tween(150)),
                ) {
                    note?.let {
                        Text(
                            it,
                            modifier = Modifier.padding(horizontal = 28.dp, vertical = 22.dp),
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                            textAlign = TextAlign.Center,
                        )
                    }
                }
            }
        }

        Box(
            Modifier
                .fillMaxWidth()
                .height(keypadHeight)
                .clipToBounds(),
        ) {
            PinKeypad(
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .fillMaxWidth()
                    .height(KEYPAD_HEIGHT),
                onDigit = onDigit,
                onBackspace = onBackspace,
                onConfirm = onConfirm,
                confirmEnabled = pinReady && !busy,
            )
        }
    }
}

/** The sliding PIN / Password segmented control. */
@Composable
private fun AuthSwitch(mode: AuthMode, onMode: (AuthMode) -> Unit) {
    val options = AuthMode.entries
    Surface(
        color = MaterialTheme.colorScheme.surfaceVariant,
        shape = CircleShape,
        modifier = Modifier.fillMaxWidth().height(46.dp),
    ) {
        BoxWithConstraints(Modifier.fillMaxSize().padding(3.dp)) {
            val itemWidth = maxWidth / options.size
            val offset by animateDpAsState(
                targetValue = itemWidth * mode.ordinal,
                animationSpec = spring(dampingRatio = 0.82f, stiffness = 520f),
                label = "auth-mode",
            )
            Box(
                Modifier
                    .offset(x = offset)
                    .width(itemWidth)
                    .fillMaxHeight()
                    .background(MaterialTheme.colorScheme.primary, CircleShape),
            )
            Row(Modifier.fillMaxSize()) {
                options.forEach { option ->
                    Box(
                        modifier = Modifier
                            .weight(1f)
                            .fillMaxHeight()
                            .clickable { onMode(option) },
                        contentAlignment = Alignment.Center,
                    ) {
                        Text(
                            option.label,
                            style = MaterialTheme.typography.labelLarge,
                            fontWeight = FontWeight.SemiBold,
                            color = if (option == mode) {
                                MaterialTheme.colorScheme.onPrimary
                            } else {
                                MaterialTheme.colorScheme.onSurfaceVariant
                            },
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun PinDots(filled: Int, slots: Int, error: Boolean) {
    // Twelve dots at the comfortable size would be wider than a 360dp phone
    // (12 x 14dp plus 11 gaps of 18dp is 366dp), so the row tightens up as the
    // PIN grows instead of running off the edge.
    val dotSize = if (slots <= 8) 14.dp else 10.dp
    val gap = if (slots <= 6) 18.dp else 10.dp
    Row(horizontalArrangement = Arrangement.spacedBy(gap)) {
        repeat(slots) { index ->
            val isFilled = index < filled
            Box(
                Modifier
                    .size(dotSize)
                    .clip(CircleShape)
                    .background(if (isFilled) MaterialTheme.colorScheme.primary else Color.Transparent)
                    .border(
                        width = 1.5.dp,
                        color = when {
                            error -> MaterialTheme.colorScheme.error
                            isFilled -> MaterialTheme.colorScheme.primary
                            else -> MaterialTheme.colorScheme.outline
                        },
                        shape = CircleShape,
                    ),
            )
        }
    }
}

@Composable
private fun PinKeypad(
    modifier: Modifier = Modifier,
    onDigit: (Char) -> Unit,
    onBackspace: () -> Unit,
    onConfirm: () -> Unit,
    confirmEnabled: Boolean,
) {
    // Confirm takes the blank slot the old keypad had; delete keeps the
    // bottom-right corner it has always had on a phone dialer.
    val rows = listOf("123", "456", "789", "✓0⌫")
    Column(
        modifier = modifier.padding(horizontal = 28.dp, vertical = 6.dp),
        verticalArrangement = Arrangement.SpaceEvenly,
    ) {
        rows.forEach { row ->
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceEvenly,
                verticalAlignment = Alignment.CenterVertically,
            ) {
                row.forEach { key ->
                    when (key) {
                        '⌫' -> KeypadKey("⌫", description = "Delete", onClick = onBackspace)
                        '✓' -> KeypadKey(
                            "✓",
                            enabled = confirmEnabled,
                            description = "Confirm PIN",
                            onClick = onConfirm,
                        )
                        else -> KeypadKey(key.toString(), onClick = { onDigit(key) })
                    }
                }
            }
        }
    }
}

@Composable
private fun KeypadKey(
    label: String,
    enabled: Boolean = true,
    description: String? = null,
    onClick: () -> Unit,
) {
    Box(
        modifier = Modifier
            .size(72.dp)
            .clip(CircleShape)
            .then(
                if (description == null) {
                    Modifier
                } else {
                    Modifier.semantics { contentDescription = description }
                },
            )
            .clickable(enabled = enabled, onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            label,
            style = MaterialTheme.typography.headlineMedium,
            fontWeight = FontWeight.Medium,
            color = if (enabled) {
                MaterialTheme.colorScheme.onSurface
            } else {
                MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.35f)
            },
        )
    }
}

@Composable
private fun PasswordField(
    label: String,
    value: String,
    onValueChange: (String) -> Unit,
) {
    OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        label = { Text(label) },
        singleLine = true,
        visualTransformation = PasswordVisualTransformation(),
        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Password),
        modifier = Modifier.fillMaxWidth(),
    )
}

@Composable
private fun RecoveryPhrase(words: String, onConfirmed: () -> Unit) {
    val parts = remember(words) { words.trim().split(Regex("\\s+")) }
    Box(Modifier.fillMaxSize().safeDrawingPadding()) {
        Column(
            modifier = Modifier
                .align(Alignment.Center)
                .fillMaxWidth()
                .verticalScroll(rememberScrollState())
                .padding(horizontal = 28.dp, vertical = 24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text(
                "Your recovery phrase",
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.SemiBold,
            )
            Spacer(Modifier.height(8.dp))
            Text(
                "Write these ${parts.size} words down in order and keep them offline.",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
            Spacer(Modifier.height(24.dp))
            Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
                parts.chunked(2).forEachIndexed { rowIndex, row ->
                    Row(
                        modifier = Modifier.fillMaxWidth(),
                        horizontalArrangement = Arrangement.spacedBy(10.dp),
                    ) {
                        row.forEachIndexed { columnIndex, word ->
                            val number = rowIndex * 2 + columnIndex + 1
                            Surface(
                                color = MaterialTheme.colorScheme.surfaceVariant,
                                shape = RoundedCornerShape(14.dp),
                                modifier = Modifier.weight(1f),
                            ) {
                                Row(
                                    modifier = Modifier.padding(horizontal = 14.dp, vertical = 14.dp),
                                    verticalAlignment = Alignment.CenterVertically,
                                ) {
                                    Text(
                                        "$number",
                                        style = MaterialTheme.typography.labelSmall,
                                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                                    )
                                    Spacer(Modifier.width(8.dp))
                                    Text(
                                        word,
                                        style = MaterialTheme.typography.titleMedium.copy(fontSize = 18.sp),
                                        fontWeight = FontWeight.Medium,
                                    )
                                }
                            }
                        }
                    }
                }
            }
            Spacer(Modifier.height(28.dp))
            Button(
                onClick = onConfirmed,
                modifier = Modifier.fillMaxWidth().height(52.dp),
            ) { Text("I've saved my recovery phrase") }
        }
    }
}
