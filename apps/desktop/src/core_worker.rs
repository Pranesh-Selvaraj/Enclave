//! The desktop's connection to the core.
//!
//! The core is a Rust crate, not a subprocess or a C ABI: the worker owns one
//! `EnclaveCore` for the app's lifetime and sends typed events to the UI. Vault
//! creation, unlock (Argon2id) and sync work stay off the UI thread.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use core_api::EnclaveCore;
use zeroize::Zeroize;

const PREVIEW_LIMIT: usize = 2_000;

/// The note summary the home screen needs. The full editor document stays in
/// the core; this is a projection, not a second model.
#[derive(Debug, Clone)]
pub struct NoteSummary {
    pub id: String,
    pub title: String,
    pub preview: String,
    pub emoji: Option<String>,
}

pub enum Command {
    CreateVault(String),
    Unlock(String),
    ListNotes,
    LoadNote(String),
    SaveDoc { document_id: String, json: String },
    RenameDocument { id: String, title: String },
    CreateNote(String),
    Lock,
    Shutdown,
}

pub enum Event {
    Initialized(bool),
    VaultCreated(String),
    Unlocked,
    Notes(Vec<NoteSummary>),
    NoteLoaded {
        id: String,
        doc_json: Option<String>,
    },
    NoteCreated {
        id: String,
    },
    TitleUpdated {
        id: String,
        title: String,
    },
    TitleError {
        id: String,
        message: String,
    },
    Saved {
        id: String,
    },
    Locked,
    Error(String),
}

pub struct CoreWorker {
    tx: Sender<Command>,
    rx: Receiver<Event>,
    join: Option<JoinHandle<()>>,
}

impl CoreWorker {
    pub fn spawn(app_dir: PathBuf, ctx: eframe::egui::Context) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::channel::<Command>();
        let (event_tx, event_rx) = mpsc::channel::<Event>();

        let join = thread::Builder::new()
            .name("enclave-core".to_owned())
            .spawn(move || {
                let _ = std::fs::create_dir_all(&app_dir);
                let core = EnclaveCore::new(app_dir);
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        let _ = event_tx.send(Event::Error(format!(
                            "could not start the core runtime: {error}"
                        )));
                        ctx.request_repaint();
                        return;
                    }
                };

                let _ = event_tx.send(Event::Initialized(core.is_vault_initialized()));
                ctx.request_repaint();

                while let Ok(command) = cmd_rx.recv() {
                    match command {
                        Command::CreateVault(mut password) => {
                            let result = core.create_vault(&password);
                            password.zeroize();
                            match result {
                                Ok(mnemonic) => {
                                    let _ = event_tx.send(Event::VaultCreated(mnemonic));
                                }
                                Err(error) => {
                                    let _ = event_tx.send(Event::Error(error.to_string()));
                                }
                            }
                        }
                        Command::Unlock(mut password) => {
                            let result = core.unlock_with_password(&password);
                            password.zeroize();
                            match result {
                                Ok(()) => {
                                    let _ = event_tx.send(Event::Unlocked);
                                }
                                Err(error) => {
                                    let _ = event_tx.send(Event::Error(error.to_string()));
                                }
                            }
                        }
                        Command::ListNotes => {
                            let _ = event_tx.send(notes_event(&core));
                        }
                        Command::LoadNote(id) => match core.get_blocks(&id) {
                            Ok(blocks) => {
                                let doc_json = blocks
                                    .into_iter()
                                    .find(|block| block.block_type == "doc")
                                    .map(|block| block.content.to_string());
                                let _ = event_tx.send(Event::NoteLoaded { id, doc_json });
                            }
                            Err(error) => {
                                let _ = event_tx.send(Event::Error(error.to_string()));
                            }
                        },
                        Command::SaveDoc { document_id, json } => {
                            let content = match serde_json::from_str::<serde_json::Value>(&json) {
                                Ok(content) => content,
                                Err(error) => {
                                    let _ = event_tx.send(Event::Error(format!(
                                        "could not serialise the page: {error}"
                                    )));
                                    ctx.request_repaint();
                                    continue;
                                }
                            };
                            match core.get_blocks(&document_id) {
                                Ok(blocks) => {
                                    let existing = blocks
                                        .iter()
                                        .find(|block| block.block_type == "doc")
                                        .or_else(|| blocks.first());
                                    let (block_id, sort_order) = existing
                                        .map(|block| (block.id.clone(), block.sort_order))
                                        .unwrap_or_else(|| (format!("{document_id}-doc"), 1.0));
                                    match core.upsert_block(
                                        &block_id,
                                        &document_id,
                                        "doc",
                                        content,
                                        sort_order,
                                    ) {
                                        Ok(_) => {
                                            let _ = event_tx.send(Event::Saved { id: document_id });
                                        }
                                        Err(error) => {
                                            let _ = event_tx.send(Event::Error(error.to_string()));
                                        }
                                    }
                                }
                                Err(error) => {
                                    let _ = event_tx.send(Event::Error(error.to_string()));
                                }
                            }
                        }
                        Command::RenameDocument { id, title } => {
                            match core.update_document_title(&id, &title) {
                                Ok(document) => {
                                    let _ = event_tx.send(Event::TitleUpdated {
                                        id: document.id,
                                        title: document.title,
                                    });
                                    let _ = event_tx.send(notes_event(&core));
                                }
                                Err(error) => {
                                    let _ = event_tx.send(Event::TitleError {
                                        id,
                                        message: error.to_string(),
                                    });
                                }
                            }
                        }
                        Command::CreateNote(title) => match core.create_document(&title) {
                            Ok(document) => {
                                let _ = event_tx.send(Event::NoteCreated { id: document.id });
                                let _ = event_tx.send(notes_event(&core));
                            }
                            Err(error) => {
                                let _ = event_tx.send(Event::Error(error.to_string()));
                            }
                        },
                        Command::Lock => match runtime.block_on(core.lock_vault()) {
                            Ok(()) => {
                                let _ = event_tx.send(Event::Locked);
                            }
                            Err(error) => {
                                let _ = event_tx.send(Event::Error(error.to_string()));
                            }
                        },
                        Command::Shutdown => break,
                    }
                    ctx.request_repaint();
                }
            })
            .expect("spawn the core worker");

        Self {
            tx: cmd_tx,
            rx: event_rx,
            join: Some(join),
        }
    }

    pub fn send(&self, command: Command) {
        let _ = self.tx.send(command);
    }

    pub fn try_recv(&self) -> Option<Event> {
        self.rx.try_recv().ok()
    }
}

fn notes_event(core: &EnclaveCore) -> Event {
    match core.home_notes() {
        Ok(rows) => {
            let notes = rows
                .into_iter()
                .map(|row| {
                    let mut preview = row
                        .doc_json
                        .as_deref()
                        .map(|json| {
                            crate::editor::ProseMirrorDoc::from_json_str(json)
                                .map(|doc| doc.plain_text())
                                .unwrap_or_else(|_| core_api::preview::plain_text(json))
                        })
                        .unwrap_or_default();
                    if preview.chars().take(PREVIEW_LIMIT + 1).count() > PREVIEW_LIMIT {
                        preview = preview.chars().take(PREVIEW_LIMIT).collect::<String>() + "…";
                    }
                    NoteSummary {
                        id: row.document.id,
                        title: row.document.title,
                        preview,
                        emoji: row.emoji,
                    }
                })
                .collect();
            Event::Notes(notes)
        }
        Err(error) => Event::Error(error.to_string()),
    }
}

impl Drop for CoreWorker {
    fn drop(&mut self) {
        let _ = self.tx.send(Command::Shutdown);
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

/// The same app-data directory the Tauri build uses, so the Rust desktop can
/// open an existing vault during the transition. `ENCLAVE_APP_DIR` overrides it
/// for tests and side-by-side development.
pub fn default_app_dir() -> PathBuf {
    if let Some(dir) = std::env::var_os("ENCLAVE_APP_DIR") {
        return PathBuf::from(dir);
    }

    let base = if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."))
    } else if cfg!(target_os = "macos") {
        home_dir()
            .map(|home| home.join("Library/Application Support"))
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join(".local/share")))
            .unwrap_or_else(|| PathBuf::from("."))
    };

    base.join("com.enclave.app")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn wait_for(worker: &CoreWorker, mut predicate: impl FnMut(&Event) -> bool) -> Event {
        for _ in 0..600 {
            if let Some(event) = worker.try_recv() {
                if predicate(&event) {
                    return event;
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("timed out waiting for a core event");
    }

    #[test]
    fn worker_creates_unlocks_lists_and_locks() {
        let dir = tempfile::tempdir().expect("temp app dir");
        let worker = CoreWorker::spawn(dir.path().to_path_buf(), eframe::egui::Context::default());

        wait_for(&worker, |event| matches!(event, Event::Initialized(false)));

        worker.send(Command::CreateVault("pass".to_owned()));
        let Event::VaultCreated(mnemonic) =
            wait_for(&worker, |event| matches!(event, Event::VaultCreated(_)))
        else {
            unreachable!()
        };
        assert_eq!(mnemonic.split_whitespace().count(), 12);

        worker.send(Command::ListNotes);
        let Event::Notes(notes) = wait_for(&worker, |event| matches!(event, Event::Notes(_)))
        else {
            unreachable!()
        };
        assert!(notes.is_empty());

        worker.send(Command::CreateNote("First note".to_owned()));
        let Event::Notes(notes) = wait_for(&worker, |event| matches!(event, Event::Notes(_)))
        else {
            unreachable!()
        };
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "First note");
        let note_id = notes[0].id.clone();

        worker.send(Command::RenameDocument {
            id: note_id.clone(),
            title: "Renamed note".to_owned(),
        });
        let Event::TitleUpdated { title, .. } =
            wait_for(&worker, |event| matches!(event, Event::TitleUpdated { .. }))
        else {
            unreachable!()
        };
        assert_eq!(title, "Renamed note");

        worker.send(Command::ListNotes);
        let Event::Notes(notes) = wait_for(&worker, |event| matches!(event, Event::Notes(_)))
        else {
            unreachable!()
        };
        assert_eq!(notes[0].title, "Renamed note");

        worker.send(Command::Lock);
        wait_for(&worker, |event| matches!(event, Event::Locked));

        worker.send(Command::Unlock("pass".to_owned()));
        wait_for(&worker, |event| matches!(event, Event::Unlocked));

        worker.send(Command::ListNotes);
        let Event::Notes(notes) = wait_for(&worker, |event| matches!(event, Event::Notes(_)))
        else {
            unreachable!()
        };
        assert_eq!(notes.len(), 1);
    }
}
