package com.enclave.app

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.activity.compose.setContent
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import com.enclave.app.core.Core
import com.enclave.app.editor.CreateKind
import com.enclave.app.editor.NoteFactory
import com.enclave.app.sync.SyncController
import com.enclave.app.ui.EnclaveTheme
import com.enclave.app.ui.HomeScreen
import com.enclave.app.ui.NoteScreen
import com.enclave.app.ui.SyncScreen
import com.enclave.app.ui.VaultScreen
import kotlinx.coroutines.launch

/** The top-level destinations. */
sealed interface Screen {
    data object Vault : Screen
    data object Home : Screen
    data class Note(val id: String) : Screen
    data object Sync : Screen
}

class MainActivity : ComponentActivity() {
    /** Set by widgets, the Quick Settings tile, shortcuts and the share sheet. */
    private var pendingEntryIntent by mutableStateOf<Intent?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Core.init(this)
        WidgetCacheBridge.install(this)
        SyncController.refresh(this)
        pendingEntryIntent = intent

        setContent {
            EnclaveTheme {
                val scope = rememberCoroutineScope()
                var screen by remember { mutableStateOf<Screen>(Screen.Vault) }

                LaunchedEffect(Unit) {
                    if (Core.ffi.isUnlocked()) {
                        SyncController.resumeIfEnabled(this@MainActivity)
                        screen = Screen.Home
                    }
                }

                // Entry points can only act once the vault is available; until
                // then the intent waits here (unlock or warm process).
                LaunchedEffect(pendingEntryIntent, screen) {
                    val entry = pendingEntryIntent ?: return@LaunchedEffect
                    if (screen != Screen.Home) return@LaunchedEffect
                    pendingEntryIntent = null
                    when (entry.action) {
                        EnclaveIntents.ACTION_NEW_NOTE -> scope.launch {
                            screen = Screen.Note(NoteFactory.create(CreateKind.TEXT).id)
                        }

                        EnclaveIntents.ACTION_NEW_CHECKLIST -> scope.launch {
                            screen = Screen.Note(NoteFactory.create(CreateKind.CHECKLIST).id)
                        }

                        EnclaveIntents.ACTION_CAPTURE -> scope.launch {
                            val text = entry.getStringExtra(EnclaveIntents.EXTRA_TEXT)
                            screen = Screen.Note(
                                NoteFactory.create(CreateKind.TEXT, prefill = text).id,
                            )
                        }

                        EnclaveIntents.ACTION_OPEN_NOTE -> {
                            entry.getStringExtra(EnclaveIntents.EXTRA_DOC_ID)
                                ?.let { screen = Screen.Note(it) }
                        }
                    }
                }

                when (val current = screen) {
                    Screen.Vault -> VaultScreen(
                        onUnlocked = {
                            SyncController.resumeIfEnabled(this@MainActivity)
                            screen = Screen.Home
                        },
                    )

                    Screen.Home -> HomeScreen(
                        onOpenNote = { screen = Screen.Note(it) },
                        onOpenSync = { screen = Screen.Sync },
                        onLock = {
                            scope.launch {
                                // Stop background sync first: the vault is about
                                // to close under it.
                                SyncController.pause(this@MainActivity)
                                Core.ffi.lockVault()
                                screen = Screen.Vault
                            }
                        },
                    )

                    is Screen.Note -> {
                        BackHandler { screen = Screen.Home }
                        NoteScreen(documentId = current.id, onBack = { screen = Screen.Home })
                    }

                    Screen.Sync -> {
                        BackHandler { screen = Screen.Home }
                        SyncScreen(onBack = { screen = Screen.Home })
                    }
                }
            }
        }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        pendingEntryIntent = intent
    }
}
