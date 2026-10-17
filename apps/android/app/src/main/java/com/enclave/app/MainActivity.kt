package com.enclave.app

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
import com.enclave.app.ui.EnclaveTheme
import com.enclave.app.ui.HomeScreen
import com.enclave.app.ui.NoteScreen
import com.enclave.app.ui.VaultScreen
import kotlinx.coroutines.launch

/** The three top-level destinations; the editor and settings hang off [Note]. */
sealed interface Screen {
    data object Vault : Screen
    data object Home : Screen
    data class Note(val id: String) : Screen
}

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        Core.init(this)

        setContent {
            EnclaveTheme {
                val scope = rememberCoroutineScope()
                var screen by remember { mutableStateOf<Screen>(Screen.Vault) }

                // A warm start (activity recreated) may already hold an unlocked
                // vault in the process-wide core.
                LaunchedEffect(Unit) {
                    if (Core.ffi.isUnlocked()) screen = Screen.Home
                }

                when (val current = screen) {
                    Screen.Vault -> VaultScreen(onUnlocked = { screen = Screen.Home })

                    Screen.Home -> HomeScreen(
                        onOpenNote = { screen = Screen.Note(it) },
                        onLock = {
                            scope.launch {
                                Core.ffi.lockVault()
                                screen = Screen.Vault
                            }
                        },
                    )

                    is Screen.Note -> {
                        BackHandler { screen = Screen.Home }
                        NoteScreen(documentId = current.id, onBack = { screen = Screen.Home })
                    }
                }
            }
        }
    }
}
