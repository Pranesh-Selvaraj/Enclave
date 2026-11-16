//! The first desktop shell: vault gate, notes list and a placeholder reading
//! pane. It is deliberately small — the next gate is the editor engine, not
//! more shell chrome.

use std::time::Duration;

use eframe::egui;
use zeroize::Zeroize;

use crate::core_worker::{Command, CoreWorker, Event, NoteSummary};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Locked,
    Mnemonic,
    Unlocked,
}

pub struct EnclaveApp {
    worker: CoreWorker,
    screen: Screen,
    initialized: bool,
    busy: bool,
    error: Option<String>,
    password: String,
    confirm: String,
    notes: Vec<NoteSummary>,
    selected: Option<String>,
    new_title: String,
    mnemonic: Option<String>,
    mnemonic_saved: bool,
}

impl EnclaveApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let app_dir = crate::core_worker::default_app_dir();
        Self {
            worker: CoreWorker::spawn(app_dir, cc.egui_ctx.clone()),
            screen: Screen::Locked,
            initialized: false,
            busy: true,
            error: None,
            password: String::new(),
            confirm: String::new(),
            notes: Vec::new(),
            selected: None,
            new_title: String::new(),
            mnemonic: None,
            mnemonic_saved: false,
        }
    }

    fn clear_secrets(&mut self) {
        self.password.zeroize();
        self.confirm.zeroize();
        self.password.clear();
        self.confirm.clear();
    }

    fn clear_mnemonic(&mut self) {
        if let Some(mut mnemonic) = self.mnemonic.take() {
            mnemonic.zeroize();
        }
    }

    fn acknowledge_mnemonic(&mut self) {
        self.screen = Screen::Unlocked;
        self.mnemonic_saved = false;
        self.busy = true;
        self.worker.send(Command::ListNotes);
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Initialized(initialized) => {
                self.initialized = initialized;
                self.busy = false;
            }
            Event::VaultCreated(mnemonic) => {
                self.clear_mnemonic();
                self.mnemonic = Some(mnemonic);
                self.screen = Screen::Mnemonic;
                self.mnemonic_saved = false;
                self.busy = false;
                self.error = None;
            }
            Event::Unlocked => {
                self.screen = Screen::Unlocked;
                self.busy = false;
                self.error = None;
                self.clear_mnemonic();
                self.clear_secrets();
                self.worker.send(Command::ListNotes);
            }
            Event::Notes(notes) => {
                self.notes = notes;
                self.busy = false;
                self.error = None;
                if let Some(selected) = &self.selected {
                    if !self.notes.iter().any(|note| &note.id == selected) {
                        self.selected = None;
                    }
                }
                if self.selected.is_none() {
                    self.selected = self.notes.first().map(|note| note.id.clone());
                }
            }
            Event::Locked => {
                self.screen = Screen::Locked;
                self.notes.clear();
                self.selected = None;
                self.clear_secrets();
                self.clear_mnemonic();
                self.mnemonic_saved = false;
                self.busy = false;
                self.error = None;
            }
            Event::Error(error) => {
                self.error = Some(error);
                self.busy = false;
            }
        }
    }

    fn locked_ui(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(72.0);
                ui.label(egui::RichText::new("Enclave").size(44.0).strong());
                ui.label("Your encrypted workspace — everything stays on this device.");
                ui.add_space(28.0);

                ui.allocate_ui_with_layout(
                    egui::vec2(380.0, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        if self.initialized {
                            ui.label("Unlock your vault");
                        } else {
                            ui.label("Create your vault");
                        }
                        ui.add(
                            egui::TextEdit::singleline(&mut self.password)
                                .password(true)
                                .hint_text("Password"),
                        );
                        if !self.initialized {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.confirm)
                                    .password(true)
                                    .hint_text("Confirm password"),
                            );
                        }

                        let mut action: Option<Command> = None;
                        ui.add_enabled_ui(!self.busy, |ui| {
                            let label = if self.initialized {
                                "Unlock"
                            } else {
                                "Create vault"
                            };
                            if ui
                                .add_sized([380.0, 36.0], egui::Button::new(label))
                                .clicked()
                            {
                                action = Some(if self.initialized {
                                    Command::Unlock(self.password.clone())
                                } else {
                                    Command::CreateVault(self.password.clone())
                                });
                            }
                        });

                        if self.busy {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Working…");
                            });
                        }

                        if let Some(command) = action {
                            if self.password.is_empty() {
                                self.error = Some("Enter a password.".to_owned());
                            } else if !self.initialized && self.password != self.confirm {
                                self.error = Some("The passwords do not match.".to_owned());
                            } else {
                                self.error = None;
                                self.busy = true;
                                self.worker.send(command);
                            }
                        }
                    },
                );
            });
        });
    }

    fn mnemonic_ui(&mut self, ui: &mut egui::Ui, mnemonic: &str) -> bool {
        let mut acknowledged = false;
        egui::CentralPanel::default().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(48.0);
                ui.heading("Save your recovery phrase");
                ui.label(
                    "This is the only way to recover your vault if you forget the password. \
                     It is shown once.",
                );
                ui.add_space(20.0);
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.set_max_width(560.0);
                    ui.horizontal_wrapped(|ui| {
                        for word in mnemonic.split_whitespace() {
                            ui.label(egui::RichText::new(word).monospace().size(18.0));
                        }
                    });
                });
                ui.add_space(16.0);
                ui.checkbox(&mut self.mnemonic_saved, "I have written down the 12 words");
                ui.add_enabled_ui(self.mnemonic_saved, |ui| {
                    if ui.button("Continue").clicked() {
                        self.acknowledge_mnemonic();
                        acknowledged = true;
                    }
                });
            });
        });
        acknowledged
    }

    fn unlocked_ui(&mut self, ui: &mut egui::Ui) {
        egui::Panel::left("notes")
            .resizable(true)
            .default_size(280.0)
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.heading("Notes");
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.new_title).hint_text("New note title"),
                    );
                    let can_create = !self.new_title.trim().is_empty() && !self.busy;
                    if ui
                        .add_enabled(can_create, egui::Button::new("Create"))
                        .clicked()
                    {
                        let title = self.new_title.trim().to_owned();
                        self.new_title.clear();
                        self.busy = true;
                        self.worker.send(Command::CreateNote(title));
                    }
                });
                if self.busy {
                    ui.spinner();
                }
                ui.separator();

                let notes = self.notes.clone();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if notes.is_empty() {
                        ui.weak("No notes yet. Create one to begin.");
                    }
                    for note in &notes {
                        let selected = self.selected.as_deref() == Some(note.id.as_str());
                        let label = match &note.emoji {
                            Some(emoji) => format!("{emoji}  {}", note.title),
                            None => note.title.clone(),
                        };
                        if ui.selectable_label(selected, label).clicked() {
                            self.selected = Some(note.id.clone());
                        }
                    }
                });
            });

        egui::CentralPanel::default().show(ui, |ui| {
            let selected = self
                .selected
                .as_ref()
                .and_then(|id| self.notes.iter().find(|note| &note.id == id))
                .cloned();

            match selected {
                Some(note) => {
                    ui.add_space(16.0);
                    ui.heading(&note.title);
                    ui.label(
                        egui::RichText::new(
                            "The native editor engine lands next. This is the core read path.",
                        )
                        .weak(),
                    );
                    ui.add_space(16.0);
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            if note.preview.is_empty() {
                                ui.weak("This page is empty.");
                            } else {
                                ui.label(note.preview);
                            }
                        });
                    });
                }
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.weak("Select a note, or create one from the sidebar.");
                    });
                }
            }
        });
    }
}

impl Drop for EnclaveApp {
    fn drop(&mut self) {
        self.password.zeroize();
        self.confirm.zeroize();
        if let Some(mnemonic) = &mut self.mnemonic {
            mnemonic.zeroize();
        }
    }
}

impl eframe::App for EnclaveApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Some(event) = self.worker.try_recv() {
            self.handle_event(event);
        }
        if self.busy {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        egui::Panel::top("top").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Enclave").strong());
                ui.label(egui::RichText::new("encrypted · local-first").weak());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if matches!(self.screen, Screen::Unlocked) && ui.button("Lock").clicked() {
                        self.busy = true;
                        self.worker.send(Command::Lock);
                    }
                });
            });
        });

        if let Some(error) = self.error.clone() {
            egui::Panel::bottom("error").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                    if ui.button("Dismiss").clicked() {
                        self.error = None;
                    }
                });
            });
        }

        match self.screen {
            Screen::Locked => self.locked_ui(ui),
            Screen::Mnemonic => {
                if let Some(mnemonic) = self.mnemonic.take() {
                    let acknowledged = self.mnemonic_ui(ui, &mnemonic);
                    if acknowledged {
                        let mut mnemonic = mnemonic;
                        mnemonic.zeroize();
                    } else {
                        self.mnemonic = Some(mnemonic);
                    }
                }
            }
            Screen::Unlocked => self.unlocked_ui(ui),
        }
    }
}
