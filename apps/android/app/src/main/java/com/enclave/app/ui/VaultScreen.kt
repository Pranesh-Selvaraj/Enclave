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

private const val PIN_LENGTH = 4
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
        if (busy || mode != AuthMode.PIN || pin.length >= PIN_LENGTH) return
        error = null
        val next = pin + digit
        if (next.length < PIN_LENGTH) {
            pin = next
            return
        }
        if (initialized == true) {
            pin = next
            return
        }
        when (stage) {
            PinStage.ENTER -> {
                firstPin = next
                pin = ""
                stage = PinStage.CONFIRM
            }
            PinStage.CONFIRM -> {
                if (next == firstPin) {
                    pin = next
                    stage = PinStage.CONFIRMED
                    keypadOpen = false
                } else {
                    pin = ""
                    firstPin = ""
                    stage = PinStage.ENTER
                    error = "PINs do not match. Try again."
                }
            }
            PinStage.CONFIRMED -> Unit
        }
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
        if (busy || stage != PinStage.CONFIRMED || firstPin.length != PIN_LENGTH) return
        scope.launch {
            busy = true
            error = null
            runCatching {
                withContext(Dispatchers.IO) { Core.ffi.createVault(firstPin) }
            }
                .onSuccess { mnemonic = it }
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
        if (mode == AuthMode.PIN && pin.length != PIN_LENGTH) return
        if (mode == AuthMode.PASSWORD && password.isEmpty()) return
        val secret = if (mode == AuthMode.PIN) pin else password
        scope.launch {
            busy = true
            error = null
            runCatching {
                withContext(Dispatchers.IO) { Core.ffi.unlockWithPassword(secret) }
            }
                .onSuccess { onUnlocked() }
                .onFailure {
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
                    lockup = lockup,
                    onDigit = ::onDigit,
                    onBackspace = ::onBackspace,
                    onOpenKeypad = ::openKeypad,
                    onRestore = {
                        restoreLauncher.launch(arrayOf("application/octet-stream", "*/*"))
                    },
                    onAction = {
                        if (initialized == true) {
                            unlock()
                        } else if (mode == AuthMode.PIN) {
                            createWithPin()
                        } else {
                            createWithPassword()
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
                                        if (looksLikeMnemonic(secret)) {
                                            // A phrase restore ends without a usable
                                            // vault.key; set a new PIN to finish.
                                            pendingMnemonic = secret
                                            newPin = ""
                                            confirmNewPin = ""
                                            pinSetupError = null
                                        } else {
                                            initialized = true
                                            mode = if (secret.length == 4 && secret.all(Char::isDigit)) {
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
                title = { Text("Set a PIN for this device") },
                text = {
                    Column {
                        Text(
                            "The vault is restored. Choose a 4-digit PIN so this device can open it.",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                        Spacer(Modifier.height(12.dp))
                        OutlinedTextField(
                            value = newPin,
                            onValueChange = {
                                newPin = it.filter(Char::isDigit).take(4)
                                pinSetupError = null
                            },
                            label = { Text("PIN") },
                            singleLine = true,
                            visualTransformation = PasswordVisualTransformation(),
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.NumberPassword),
                            modifier = Modifier.fillMaxWidth(),
                        )
                        Spacer(Modifier.height(8.dp))
                        OutlinedTextField(
                            value = confirmNewPin,
                            onValueChange = {
                                confirmNewPin = it.filter(Char::isDigit).take(4)
                                pinSetupError = null
                            },
                            label = { Text("Confirm PIN") },
                            singleLine = true,
                            visualTransformation = PasswordVisualTransformation(),
                            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.NumberPassword),
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
                        enabled = !pinSetupBusy && newPin.length == 4 && newPin == confirmNewPin,
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
                                        initialized = true
                                        mode = AuthMode.PIN
                                        AppPrefs.setAuthMode(context, AuthMode.PIN)
                                        restoreNotice = "Vault restored — unlock with your new PIN."
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
    lockup: Painter,
    onDigit: (Char) -> Unit,
    onBackspace: () -> Unit,
    onOpenKeypad: () -> Unit,
    onRestore: () -> Unit,
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
    val pinConfirmed = stage == PinStage.CONFIRMED
    val pinReady = pin.length == PIN_LENGTH
    val passwordReady = password.isNotEmpty() && (!isRegister || password == confirmPassword)

    val pinLabel = when {
        !isRegister -> "Enter your PIN"
        stage == PinStage.ENTER -> "Create a PIN"
        stage == PinStage.CONFIRM -> "Confirm your PIN"
        else -> "PIN confirmed"
    }
    val dots = if (pinConfirmed) PIN_LENGTH else pin.length
    val note = if (isRegister) {
        "A 12-word recovery phrase is generated next. It is the only way to recover the vault if you forget your ${if (mode == AuthMode.PIN) "PIN" else "password"} — write it down and keep it offline."
    } else {
        null
    }

    val actionEnabled = when {
        busy -> false
        initialized && mode == AuthMode.PIN -> pinReady
        initialized -> passwordReady
        mode == AuthMode.PIN -> pinConfirmed
        else -> passwordReady
    }
    val actionLabel = when {
        busy && initialized -> "Unlocking…"
        busy -> "Creating…"
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
                        PinDots(count = dots, error = error != null)
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
private fun PinDots(count: Int, error: Boolean) {
    Row(horizontalArrangement = Arrangement.spacedBy(18.dp)) {
        repeat(PIN_LENGTH) { index ->
            val filled = index < count
            Box(
                Modifier
                    .size(14.dp)
                    .clip(CircleShape)
                    .background(if (filled) MaterialTheme.colorScheme.primary else Color.Transparent)
                    .border(
                        width = 1.5.dp,
                        color = when {
                            error -> MaterialTheme.colorScheme.error
                            filled -> MaterialTheme.colorScheme.primary
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
) {
    val rows = listOf("123", "456", "789", " 0⌫")
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
                        ' ' -> Spacer(Modifier.size(72.dp))
                        '⌫' -> KeypadKey("⌫", onBackspace)
                        else -> KeypadKey(key.toString()) { onDigit(key) }
                    }
                }
            }
        }
    }
}

@Composable
private fun KeypadKey(label: String, onClick: () -> Unit) {
    Box(
        modifier = Modifier
            .size(72.dp)
            .clip(CircleShape)
            .clickable(onClick = onClick),
        contentAlignment = Alignment.Center,
    ) {
        Text(
            label,
            style = MaterialTheme.typography.headlineMedium,
            fontWeight = FontWeight.Medium,
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
