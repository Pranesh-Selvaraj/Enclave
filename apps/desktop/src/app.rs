//! The first desktop shell: vault gate, notes list and an editable document
//! view. Text blocks are bound to the headless engine, autosaved through the
//! core; IME, accessibility and table-cell polish are the next gate.

use std::time::{Duration, Instant};

use eframe::egui;
use zeroize::Zeroize;

use crate::core_worker::{Command, CoreWorker, Event, NoteSummary};
use crate::editor::{
    Editor, NodePath, ProseMirrorDoc, ProseMirrorMark, ProseMirrorNode, Selection,
};

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
    editor: Option<Editor>,
    loading_note: Option<String>,
    dirty: bool,
    saving: bool,
    pending_save: Option<(String, String)>,
    last_edit: Option<Instant>,
    title_edit: String,
    title_note_id: Option<String>,
    title_dirty: bool,
    title_in_flight: Option<String>,
    title_last_edit: Option<Instant>,
    focused_block: Option<NodePath>,
    focused_range: Option<(usize, usize)>,
    pending_focus: Option<NodePath>,
    new_title: String,
    composer_open: bool,
    composer_focus: bool,
    mnemonic: Option<String>,
    glow_texture: egui::TextureHandle,
    started_at: Instant,
    initialized_known: bool,
    gate_form_visible: bool,
    gate_focus_pending: bool,
}

impl EnclaveApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::brand::install(&cc.egui_ctx);
        egui_extras::install_image_loaders(&cc.egui_ctx);
        let glow_texture = crate::brand::load_glow(&cc.egui_ctx);
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
            editor: None,
            loading_note: None,
            dirty: false,
            saving: false,
            pending_save: None,
            last_edit: None,
            title_edit: String::new(),
            title_note_id: None,
            title_dirty: false,
            title_in_flight: None,
            title_last_edit: None,
            focused_block: None,
            focused_range: None,
            pending_focus: None,
            new_title: String::new(),
            composer_open: false,
            composer_focus: false,
            mnemonic: None,
            glow_texture,
            started_at: Instant::now(),
            initialized_known: false,
            gate_form_visible: false,
            gate_focus_pending: false,
        }
    }

    fn clear_secrets(&mut self) {
        self.password.zeroize();
        self.confirm.zeroize();
        self.password.clear();
        self.confirm.clear();
    }

    fn gate_ready(&self) -> bool {
        !self.busy
            && !self.password.is_empty()
            && (self.initialized || self.password == self.confirm)
    }

    fn submit_gate(&mut self) {
        if self.busy {
            return;
        }
        if self.password.is_empty() {
            self.error = Some("Enter a password.".to_owned());
            return;
        }
        if !self.initialized && self.password != self.confirm {
            self.error = Some("The passwords do not match.".to_owned());
            return;
        }
        self.error = None;
        self.busy = true;
        if self.initialized {
            self.worker.send(Command::Unlock(self.password.clone()));
        } else {
            self.worker
                .send(Command::CreateVault(self.password.clone()));
        }
    }

    fn clear_mnemonic(&mut self) {
        if let Some(mut mnemonic) = self.mnemonic.take() {
            mnemonic.zeroize();
        }
    }

    fn clear_title_state(&mut self) {
        self.title_edit.clear();
        self.title_note_id = None;
        self.title_dirty = false;
        self.title_in_flight = None;
        self.title_last_edit = None;
    }

    fn flush_title(&mut self) {
        if self.title_in_flight.is_some() || !self.title_dirty {
            return;
        }
        let Some(id) = self.title_note_id.clone() else {
            return;
        };
        let title = self.title_edit.trim().to_owned();
        self.title_dirty = false;
        self.title_in_flight = Some(title.clone());
        self.worker.send(Command::RenameDocument { id, title });
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
        self.last_edit = Some(Instant::now());
    }

    fn save_selected(&mut self) {
        if self.saving || !self.dirty {
            return;
        }
        let (Some(id), Some(editor)) = (self.selected.clone(), self.editor.as_ref()) else {
            return;
        };
        self.saving = true;
        self.dirty = false;
        let json = editor.to_json_string();
        self.pending_save = Some((id.clone(), json.clone()));
        self.worker.send(Command::SaveDoc {
            document_id: id,
            json,
        });
    }

    fn save_before_switch(&mut self) {
        if self.saving || !self.dirty {
            return;
        }
        let (Some(id), Some(editor)) = (self.selected.clone(), self.editor.as_ref()) else {
            return;
        };
        self.saving = true;
        self.dirty = false;
        let json = editor.to_json_string();
        self.pending_save = Some((id.clone(), json.clone()));
        self.worker.send(Command::SaveDoc {
            document_id: id,
            json,
        });
    }

    fn select_note(&mut self, id: String) {
        if self.selected.as_deref() == Some(id.as_str()) && self.editor.is_some() {
            return;
        }
        self.flush_title();
        self.save_before_switch();
        self.clear_title_state();
        self.selected = Some(id.clone());
        self.editor = None;
        self.loading_note = Some(id.clone());
        self.focused_block = None;
        self.focused_range = None;
        self.pending_focus = None;
        self.busy = true;
        self.worker.send(Command::LoadNote(id));
    }

    fn load_selected(&mut self) {
        if self.loading_note.is_some() || self.editor.is_some() {
            return;
        }
        if let Some(id) = self.selected.clone() {
            self.loading_note = Some(id.clone());
            self.busy = true;
            self.worker.send(Command::LoadNote(id));
        }
    }

    fn editor_tools(&mut self, ui: &mut egui::Ui) {
        if self.editor.is_none() {
            return;
        }
        let mut undo = false;
        let mut redo = false;
        let mut save = false;
        let mut mark: Option<&'static str> = None;
        let mut list_type: Option<&'static str> = None;
        egui::Panel::bottom("editor-tools").show(ui, |ui| {
            ui.horizontal(|ui| {
                let can_undo = self.editor.as_ref().is_some_and(Editor::can_undo);
                let can_redo = self.editor.as_ref().is_some_and(Editor::can_redo);
                if ui.add_enabled(can_undo, egui::Button::new("↶")).clicked() {
                    undo = true;
                }
                if ui.add_enabled(can_redo, egui::Button::new("↷")).clicked() {
                    redo = true;
                }
                ui.separator();
                let can_list = self.focused_block.is_some();
                if ui
                    .add_enabled(can_list, egui::Button::new("• List"))
                    .on_hover_text("Bullet list")
                    .clicked()
                {
                    list_type = Some("bulletList");
                }
                if ui
                    .add_enabled(can_list, egui::Button::new("1. List"))
                    .on_hover_text("Ordered list")
                    .clicked()
                {
                    list_type = Some("orderedList");
                }
                if ui
                    .add_enabled(can_list, egui::Button::new("☑ Task"))
                    .on_hover_text("Task list")
                    .clicked()
                {
                    list_type = Some("taskList");
                }
                ui.separator();
                let can_mark = self.focused_range.is_some_and(|(start, end)| start < end);
                if ui
                    .add_enabled(
                        can_mark,
                        egui::Button::new(egui::RichText::new("B").strong()),
                    )
                    .clicked()
                {
                    mark = Some("bold");
                }
                if ui
                    .add_enabled(
                        can_mark,
                        egui::Button::new(egui::RichText::new("I").italics()),
                    )
                    .clicked()
                {
                    mark = Some("italic");
                }
                if ui
                    .add_enabled(
                        can_mark,
                        egui::Button::new(egui::RichText::new("S").strikethrough()),
                    )
                    .clicked()
                {
                    mark = Some("strike");
                }
                if ui
                    .add_enabled(
                        can_mark,
                        egui::Button::new(egui::RichText::new("</>").monospace()),
                    )
                    .clicked()
                {
                    mark = Some("code");
                }
                ui.separator();
                if self.saving {
                    ui.label("Saving…");
                } else if self.dirty {
                    ui.label("● Unsaved");
                } else {
                    ui.weak("Saved");
                }
                if ui
                    .add_enabled(self.dirty && !self.saving, egui::Button::new("Save"))
                    .clicked()
                {
                    save = true;
                }
            });
        });

        if let Some(list_type) = list_type {
            let offset = self.focused_range.map(|(start, _)| start).unwrap_or(0);
            let mut changed = false;
            if let (Some(path), Some(editor)) = (self.focused_block.clone(), self.editor.as_mut()) {
                editor.set_selection(Selection::caret(path, offset));
                changed = editor.set_list_type(list_type);
                if changed {
                    self.pending_focus = Some(editor.selection().path.clone());
                }
            }
            if changed {
                self.mark_dirty();
            }
        }
        if undo {
            if let Some(editor) = self.editor.as_mut() {
                if editor.undo() {
                    self.mark_dirty();
                }
            }
        }
        if redo {
            if let Some(editor) = self.editor.as_mut() {
                if editor.redo() {
                    self.mark_dirty();
                }
            }
        }
        if let Some(mark_type) = mark {
            if let (Some(path), Some((start, end))) =
                (self.focused_block.clone(), self.focused_range)
            {
                if start < end {
                    if let Some(editor) = self.editor.as_mut() {
                        editor.set_selection(Selection::range(path, start, end));
                        if editor.toggle_mark(ProseMirrorMark::new(mark_type)) {
                            self.mark_dirty();
                        }
                    }
                }
            }
        }
        if save {
            self.save_selected();
        }
    }

    fn apply_actions(&mut self, actions: Vec<EditorAction>) {
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        let mut changed = false;
        for action in actions {
            changed |= match action {
                EditorAction::Replace { path, text } => replace_text(editor, &path, &text),
                EditorAction::Split { path, offset } => {
                    editor.set_selection(Selection::caret(path, offset));
                    let changed = editor.split_block();
                    if changed {
                        self.pending_focus = Some(editor.selection().path.clone());
                    }
                    changed
                }
                EditorAction::MergeBackward { path } => {
                    editor.set_selection(Selection::caret(path, 0));
                    let changed = editor.delete_backward();
                    if changed {
                        self.pending_focus = Some(editor.selection().path.clone());
                    }
                    changed
                }
                EditorAction::ToggleTask { path } => {
                    editor.set_selection(Selection::caret(path, 0));
                    editor.toggle_task()
                }
                EditorAction::Indent { path } => {
                    editor.set_selection(Selection::caret(path, 0));
                    let changed = editor.indent_list_item();
                    if changed {
                        self.pending_focus = Some(editor.selection().path.clone());
                    }
                    changed
                }
                EditorAction::Outdent { path } => {
                    editor.set_selection(Selection::caret(path, 0));
                    let changed = editor.outdent_list_item();
                    if changed {
                        self.pending_focus = Some(editor.selection().path.clone());
                    }
                    changed
                }
            };
        }
        if changed {
            self.mark_dirty();
        }
    }

    fn acknowledge_mnemonic(&mut self) {
        self.screen = Screen::Unlocked;
        self.busy = true;
        self.worker.send(Command::ListNotes);
    }

    fn handle_event(&mut self, event: Event) {
        match event {
            Event::Initialized(initialized) => {
                self.initialized = initialized;
                self.initialized_known = true;
                self.busy = false;
            }
            Event::VaultCreated(mnemonic) => {
                self.clear_mnemonic();
                self.mnemonic = Some(mnemonic);
                self.screen = Screen::Mnemonic;
                self.busy = false;
                self.error = None;
            }
            Event::Unlocked => {
                self.screen = Screen::Unlocked;
                self.busy = false;
                self.error = None;
                self.editor = None;
                self.loading_note = None;
                self.clear_title_state();
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
                        self.editor = None;
                    }
                }
                if self.selected.is_none() {
                    self.selected = self.notes.first().map(|note| note.id.clone());
                }
                if !self.title_dirty && self.title_in_flight.is_none() {
                    if let Some(selected) = &self.selected {
                        if let Some(note) = self.notes.iter().find(|note| &note.id == selected) {
                            self.title_edit = note.title.clone();
                            self.title_note_id = Some(selected.clone());
                        }
                    }
                }
                self.load_selected();
            }
            Event::NoteLoaded { id, doc_json } => {
                if self.selected.as_deref() != Some(id.as_str()) {
                    return;
                }
                self.loading_note = None;
                self.busy = false;
                self.error = None;
                self.title_edit = self
                    .notes
                    .iter()
                    .find(|note| note.id == id)
                    .map(|note| note.title.clone())
                    .unwrap_or_default();
                self.title_note_id = Some(id.clone());
                self.title_dirty = false;
                self.title_in_flight = None;
                self.title_last_edit = None;
                self.dirty = false;
                self.saving = false;
                self.pending_save = None;
                self.last_edit = None;
                self.focused_block = None;
                self.focused_range = None;
                self.pending_focus = None;
                self.editor = match doc_json {
                    Some(json) => match Editor::from_json_str(&json) {
                        Ok(editor) => Some(editor),
                        Err(error) => {
                            self.error = Some(format!("This page could not be opened: {error}"));
                            None
                        }
                    },
                    None => Some(Editor::new(ProseMirrorDoc {
                        node_type: "doc".to_owned(),
                        content: vec![ProseMirrorNode::paragraph(Vec::new())],
                    })),
                };
            }
            Event::NoteCreated { id } => {
                self.busy = false;
                self.composer_open = false;
                self.composer_focus = false;
                self.select_note(id);
            }
            Event::TitleUpdated { id, title } => {
                if let Some(note) = self.notes.iter_mut().find(|note| note.id == id) {
                    note.title = title.clone();
                }
                if self.title_in_flight.as_deref() == Some(title.as_str()) {
                    self.title_in_flight = None;
                }
                if self.selected.as_deref() == Some(id.as_str()) {
                    self.title_note_id = Some(id);
                    if !self.title_dirty {
                        self.title_edit = title;
                    }
                }
            }
            Event::TitleError { id, message } => {
                self.title_in_flight = None;
                self.error = Some(format!("Could not rename the note: {message}"));
                if self.selected.as_deref() == Some(id.as_str()) {
                    self.title_dirty = true;
                }
            }
            Event::Saved { id } => {
                if self
                    .pending_save
                    .as_ref()
                    .is_some_and(|(pending_id, _)| pending_id == &id)
                {
                    self.pending_save = None;
                }
                self.saving = false;
                self.error = None;
            }
            Event::Locked => {
                self.screen = Screen::Locked;
                self.notes.clear();
                self.selected = None;
                self.editor = None;
                self.loading_note = None;
                self.clear_title_state();
                self.dirty = false;
                self.saving = false;
                self.pending_save = None;
                self.last_edit = None;
                self.focused_block = None;
                self.focused_range = None;
                self.pending_focus = None;
                self.clear_secrets();
                self.clear_mnemonic();
                self.gate_form_visible = true;
                self.gate_focus_pending = true;
                self.busy = false;
                self.error = None;
            }
            Event::Error(error) => {
                let was_saving = self.saving;
                self.error = Some(error);
                self.busy = false;
                self.loading_note = None;
                if was_saving {
                    self.saving = false;
                    self.dirty = true;
                    self.last_edit = None;
                    if let Some((id, json)) = self.pending_save.take() {
                        if self.selected.as_deref() != Some(id.as_str()) {
                            self.selected = Some(id);
                            self.editor = Editor::from_json_str(&json).ok();
                            self.loading_note = None;
                        }
                    }
                }
            }
        }
    }

    fn locked_ui(&mut self, ui: &mut egui::Ui) {
        if !self.gate_form_visible
            && (self.initialized_known || self.error.is_some())
            && self.started_at.elapsed() >= Duration::from_millis(900)
        {
            self.gate_form_visible = true;
            self.gate_focus_pending = true;
        }
        let form_progress = ui.ctx().animate_bool_with_time(
            egui::Id::unique("enclave-gate-form"),
            self.gate_form_visible,
            0.55,
        );
        let appeared =
            ui.ctx()
                .animate_bool_with_time(egui::Id::unique("enclave-gate-lockup"), true, 0.6);

        let glow_texture = self.glow_texture.id();
        let mut submitted = false;
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(crate::brand::BG))
            .show(ui, |ui| {
                // Centre the lockup on its own at launch, then let the form
                // grow around it while the whole group stays optically centred.
                let hidden_height = 360.0 / crate::brand::LOCKUP_RATIO + 42.0;
                let form_height = if self.initialized { 310.0 } else { 410.0 };
                let shown_height =
                    260.0 / crate::brand::LOCKUP_RATIO + 42.0 + 26.0 + form_height;
                let content_height =
                    hidden_height + (shown_height - hidden_height) * form_progress;
                let top = ((ui.available_height() - content_height) * 0.5).max(24.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add_space(top);
                ui.vertical_centered(|ui| {
                    let width = 360.0 - 100.0 * form_progress;
                    crate::brand::lockup(
                        ui,
                        glow_texture,
                        width * (0.94 + 0.06 * appeared),
                        0.16 * (1.0 - form_progress),
                        appeared,
                    );
                    ui.add_space(14.0);
                    ui.label(
                        egui::RichText::new("Knowledge that stays yours.")
                            .size(13.5)
                            .color(crate::brand::MUTED.gamma_multiply(1.0 - 0.35 * form_progress)),
                    );

                    if form_progress > 0.01 {
                        ui.add_space(26.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(380.0, 0.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.set_opacity(form_progress);
                                ui.label(
                                    egui::RichText::new(if self.initialized {
                                        "Welcome back"
                                    } else {
                                        "Create your vault"
                                    })
                                    .size(25.0)
                                    .family(crate::brand::semibold())
                                    .color(crate::brand::PAPER),
                                );
                                ui.add_space(5.0);
                                ui.label(
                                    egui::RichText::new(if self.initialized {
                                        "Unlock your vault to continue."
                                    } else {
                                        "A private place for everything you write."
                                    })
                                    .size(14.0)
                                    .color(crate::brand::MUTED),
                                );
                                ui.add_space(20.0);

                                let password_id = egui::Id::unique("gate-password");
                                let password = crate::brand::field(
                                    ui,
                                    password_id,
                                    if self.initialized {
                                        "Password"
                                    } else {
                                        "Choose a password"
                                    },
                                    &mut self.password,
                                    true,
                                );
                                if password.changed() {
                                    self.error = None;
                                }
                                if !self.initialized {
                                    let confirm = crate::brand::field(
                                        ui,
                                        egui::Id::unique("gate-confirm"),
                                        "Confirm password",
                                        &mut self.confirm,
                                        true,
                                    );
                                    if confirm.changed() {
                                        self.error = None;
                                    }
                                }

                                let mismatch = !self.initialized
                                    && !self.confirm.is_empty()
                                    && self.password != self.confirm;
                                let visible_error = if mismatch {
                                    Some("The passwords do not match.".to_owned())
                                } else {
                                    self.error.clone()
                                };
                                if let Some(error) = visible_error {
                                    ui.add_space(2.0);
                                    ui.label(
                                        egui::RichText::new(error)
                                            .size(12.5)
                                            .color(crate::brand::ERROR),
                                    );
                                    ui.add_space(6.0);
                                }

                                let ready = self.gate_ready() && !mismatch;
                                let label = if self.busy {
                                    if self.initialized {
                                        "Unlocking…"
                                    } else {
                                        "Creating…"
                                    }
                                } else if self.initialized {
                                    "Unlock"
                                } else {
                                    "Create vault"
                                };
                                if crate::brand::primary_button(ui, label, ready, 380.0) {
                                    submitted = true;
                                }
                                if !self.initialized {
                                    ui.add_space(12.0);
                                    ui.label(
                                        egui::RichText::new(
                                            "A 12-word recovery phrase comes next. Keep it offline — it is the only way back if you forget your password.",
                                        )
                                        .size(12.0)
                                        .color(crate::brand::MUTED),
                                    );
                                }
                                if self.gate_focus_pending {
                                    ui.memory_mut(|memory| memory.request_focus(password_id));
                                    self.gate_focus_pending = false;
                                }
                                if ui.input(|input| input.key_pressed(egui::Key::Enter)) && ready {
                                    submitted = true;
                                }
                            },
                        );
                    }
                });
                });
            });
        if submitted {
            self.submit_gate();
        }
    }

    fn mnemonic_ui(&mut self, ui: &mut egui::Ui, mnemonic: &str) -> bool {
        let mut acknowledged = false;
        let words: Vec<&str> = mnemonic.split_whitespace().collect();
        let reveal = ui.ctx().animate_bool_with_time_and_easing(
            egui::Id::unique("enclave-mnemonic-reveal"),
            true,
            0.35,
            egui::emath::easing::cubic_out,
        );
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(crate::brand::BG))
            .show(ui, |ui| {
                ui.set_opacity(reveal);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let top = (ui.available_height() * 0.10).clamp(24.0, 72.0);
                    ui.add_space(top);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Your recovery phrase")
                                .size(26.0)
                                .family(crate::brand::semibold())
                                .color(crate::brand::PAPER),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(format!(
                                "Write these {} words down in order and keep them offline.",
                                words.len()
                            ))
                            .size(14.0)
                            .color(crate::brand::MUTED),
                        );
                        ui.add_space(22.0);
                        let width = ui.available_width().min(560.0);
                        ui.allocate_ui_with_layout(
                            egui::vec2(width, 0.0),
                            egui::Layout::top_down(egui::Align::Center),
                            |ui| {
                                let card_width = (width - 10.0) / 2.0;
                                for (row_index, row) in words.chunks(2).enumerate() {
                                    ui.horizontal(|ui| {
                                        for (column_index, word) in row.iter().enumerate() {
                                            let number = row_index * 2 + column_index + 1;
                                            egui::Frame::new()
                                                .fill(crate::brand::SURFACE_VARIANT)
                                                .corner_radius(12)
                                                .inner_margin(egui::Margin::symmetric(14, 12))
                                                .show(ui, |ui| {
                                                    ui.set_min_size(egui::vec2(
                                                        card_width - 28.0,
                                                        22.0,
                                                    ));
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(number.to_string())
                                                                .size(11.0)
                                                                .color(crate::brand::MUTED),
                                                        );
                                                        ui.add_space(4.0);
                                                        ui.label(
                                                            egui::RichText::new(*word)
                                                                .size(17.0)
                                                                .family(crate::brand::medium())
                                                                .color(crate::brand::PAPER),
                                                        );
                                                    });
                                                });
                                        }
                                    });
                                    ui.add_space(10.0);
                                }
                            },
                        );
                        ui.add_space(8.0);
                        if crate::brand::primary_button(
                            ui,
                            "I've saved my recovery phrase",
                            true,
                            width.min(520.0),
                        ) {
                            acknowledged = true;
                        }
                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new(
                                "This phrase is shown once. Keep it somewhere offline.",
                            )
                            .size(12.0)
                            .color(crate::brand::MUTED),
                        );
                    });
                });
            });
        acknowledged
    }

    fn unlocked_ui(&mut self, ui: &mut egui::Ui) {
        self.editor_tools(ui);
        egui::Panel::left("notes")
            .resizable(false)
            .default_size(280.0)
            .show(ui, |ui| {
                ui.add_space(8.0);
                ui.heading("Notes");
                if ui.button("+ New note").clicked() {
                    self.composer_open = true;
                    self.composer_focus = true;
                    self.new_title.clear();
                }
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
                            self.select_note(note.id.clone());
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
                Some(_) if self.editor.is_some() => {
                    ui.add_space(12.0);
                    let mut title = self.title_edit.clone();
                    let title_response = ui.add(
                        egui::TextEdit::singleline(&mut title)
                            .font(egui::TextStyle::Heading)
                            .frame(egui::Frame::NONE)
                            .desired_width(f32::INFINITY),
                    );
                    if title_response.changed() {
                        self.title_edit = title;
                        self.title_dirty = true;
                        self.title_last_edit = Some(Instant::now());
                    }
                    if title_response.lost_focus() {
                        self.flush_title();
                    }
                    if self.dirty || self.title_dirty {
                        ui.weak("● unsaved");
                    }
                    ui.separator();
                    let ime_active = ui.input(|input| {
                        input
                            .events
                            .iter()
                            .any(|event| matches!(event, egui::Event::Ime(_)))
                    });
                    let view_input = ViewInput {
                        enter: !ime_active
                            && ui.input(|input| {
                                input.key_pressed(egui::Key::Enter) && input.modifiers.is_none()
                            }),
                        backspace: !ime_active
                            && ui.input(|input| {
                                input.key_pressed(egui::Key::Backspace) && input.modifiers.is_none()
                            }),
                        indent: !ime_active
                            && ui.input(|input| {
                                input.key_pressed(egui::Key::Tab) && !input.modifiers.shift
                            }),
                        outdent: !ime_active
                            && ui.input(|input| {
                                input.key_pressed(egui::Key::Tab) && input.modifiers.shift
                            }),
                    };
                    let mut actions = Vec::new();
                    let mut focus = ViewFocus::default();
                    let mut focus_request = self.pending_focus.take();
                    if let Some(editor) = &self.editor {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            let mut path = Vec::new();
                            for (index, node) in editor.doc().content.iter().enumerate() {
                                path.push(index);
                                render_block(
                                    ui,
                                    node,
                                    0,
                                    &mut path,
                                    &mut actions,
                                    &view_input,
                                    &mut focus,
                                    &mut focus_request,
                                );
                                path.pop();
                            }
                        });
                    }
                    self.pending_focus = None;
                    self.apply_actions(actions);
                    self.focused_block = focus.path;
                    self.focused_range = focus.range;
                }
                Some(note) => {
                    ui.add_space(16.0);
                    ui.heading(&note.title);
                    ui.label(egui::RichText::new("Loading the page through the core…").weak());
                    if !note.preview.is_empty() {
                        ui.add_space(16.0);
                        egui::Frame::group(ui.style()).show(ui, |ui| {
                            ui.label(note.preview);
                        });
                    }
                }
                None => {
                    ui.centered_and_justified(|ui| {
                        ui.weak("Select a note, or create one from the sidebar.");
                    });
                }
            }
        });

        if self.composer_open {
            let mut create = false;
            let mut cancel = false;
            egui::Window::new("New note")
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    ui.label("Title");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.new_title).hint_text("Note title"),
                    );
                    if self.composer_focus {
                        response.request_focus();
                        self.composer_focus = false;
                    }
                    if response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter))
                    {
                        create = true;
                    }
                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            create = true;
                        }
                        if ui.button("Cancel").clicked() {
                            cancel = true;
                        }
                    });
                });
            if create && !self.new_title.trim().is_empty() {
                let title = self.new_title.trim().to_owned();
                self.new_title.clear();
                self.composer_open = false;
                self.busy = true;
                self.worker.send(Command::CreateNote(title));
            }
            if cancel {
                self.composer_open = false;
                self.new_title.clear();
            }
        }
    }
}

impl Drop for EnclaveApp {
    fn drop(&mut self) {
        if self.title_dirty {
            if let Some(id) = self.title_note_id.clone() {
                self.worker.send(Command::RenameDocument {
                    id,
                    title: self.title_edit.trim().to_owned(),
                });
            }
        }
        if self.dirty && !self.saving {
            if let (Some(id), Some(editor)) = (self.selected.clone(), self.editor.as_ref()) {
                self.worker.send(Command::SaveDoc {
                    document_id: id,
                    json: editor.to_json_string(),
                });
            }
        }
        self.password.zeroize();
        self.confirm.zeroize();
        if let Some(mnemonic) = &mut self.mnemonic {
            mnemonic.zeroize();
        }
    }
}

impl eframe::App for EnclaveApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        crate::brand::BG.to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Some(event) = self.worker.try_recv() {
            self.handle_event(event);
        }
        if ui.input(|input| input.key_pressed(egui::Key::S) && input.modifiers.command) {
            self.save_selected();
        }
        if self.dirty
            && !self.saving
            && self
                .last_edit
                .is_some_and(|edited| edited.elapsed() >= Duration::from_millis(700))
        {
            self.save_selected();
        }
        if self.title_dirty
            && self.title_in_flight.is_none()
            && self
                .title_last_edit
                .is_some_and(|edited| edited.elapsed() >= Duration::from_millis(700))
        {
            self.flush_title();
        }
        if self.busy
            || self.dirty
            || self.saving
            || self.title_dirty
            || self.title_in_flight.is_some()
        {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }

        if matches!(self.screen, Screen::Unlocked) {
            egui::Panel::top("top").show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Enclave").strong());
                    ui.label(egui::RichText::new("encrypted · local-first").weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Lock").clicked() {
                            self.flush_title();
                            self.save_selected();
                            self.busy = true;
                            self.worker.send(Command::Lock);
                        }
                    });
                });
            });

            if let Some(error) = self.error.clone() {
                egui::Panel::bottom("error").show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(crate::brand::ERROR, error);
                        if ui.button("Dismiss").clicked() {
                            self.error = None;
                        }
                    });
                });
            }
        }

        if self.screen == Screen::Locked && !self.gate_form_visible {
            ui.ctx().request_repaint_after(Duration::from_millis(50));
        }

        match self.screen {
            Screen::Locked => self.locked_ui(ui),
            Screen::Mnemonic => {
                if let Some(mnemonic) = self.mnemonic.take() {
                    let acknowledged = self.mnemonic_ui(ui, &mnemonic);
                    if acknowledged {
                        let mut mnemonic = mnemonic;
                        mnemonic.zeroize();
                        self.acknowledge_mnemonic();
                    } else {
                        self.mnemonic = Some(mnemonic);
                    }
                }
            }
            Screen::Unlocked => self.unlocked_ui(ui),
        }
    }
}

fn attr_str<'a>(node: &'a ProseMirrorNode, key: &str) -> Option<&'a str> {
    node.attrs.as_ref()?.get(key)?.as_str()
}

fn attr_bool(node: &ProseMirrorNode, key: &str) -> Option<bool> {
    node.attrs.as_ref()?.get(key)?.as_bool()
}

fn attr_i64(node: &ProseMirrorNode, key: &str) -> Option<i64> {
    node.attrs.as_ref()?.get(key)?.as_i64()
}

#[derive(Clone)]
enum EditorAction {
    Replace { path: NodePath, text: String },
    Split { path: NodePath, offset: usize },
    MergeBackward { path: NodePath },
    ToggleTask { path: NodePath },
    Indent { path: NodePath },
    Outdent { path: NodePath },
}

#[derive(Default)]
struct ViewFocus {
    path: Option<NodePath>,
    range: Option<(usize, usize)>,
}

#[derive(Clone, Copy, Default)]
struct ViewInput {
    enter: bool,
    backspace: bool,
    indent: bool,
    outdent: bool,
}

/// The view reports a whole new string; replace only the changed span so marks
/// outside it survive. Newlines are not valid inside a paragraph, so they are
/// folded to spaces; code blocks keep them.
fn replace_text(editor: &mut Editor, path: &NodePath, next: &str) -> bool {
    let Some(node) = editor.doc().node(path) else {
        return false;
    };
    let old = node.inline_text();
    let next = if node.node_type == "codeBlock" {
        next.to_owned()
    } else {
        next.replace('\n', " ")
    };
    if old == next {
        return false;
    }
    let old_chars: Vec<char> = old.chars().collect();
    let next_chars: Vec<char> = next.chars().collect();
    let limit = old_chars.len().min(next_chars.len());
    let mut prefix = 0;
    while prefix < limit && old_chars[prefix] == next_chars[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old_chars.len() - prefix
        && suffix < next_chars.len() - prefix
        && old_chars[old_chars.len() - 1 - suffix] == next_chars[next_chars.len() - 1 - suffix]
    {
        suffix += 1;
    }
    let end = old_chars.len() - suffix;
    let inserted: String = next_chars[prefix..next_chars.len() - suffix]
        .iter()
        .collect();
    editor.replace_range(path.clone(), prefix, end, &inserted)
}

#[allow(clippy::too_many_arguments)]
fn render_block(
    ui: &mut egui::Ui,
    node: &ProseMirrorNode,
    depth: usize,
    path: &mut Vec<usize>,
    actions: &mut Vec<EditorAction>,
    input: &ViewInput,
    focus: &mut ViewFocus,
    focus_request: &mut Option<NodePath>,
) {
    match node.node_type.as_str() {
        "heading" => {
            let level = attr_i64(node, "level").unwrap_or(1).clamp(1, 6);
            let size = match level {
                1 => 30.0,
                2 => 24.0,
                3 => 20.0,
                _ => 18.0,
            };
            render_text_block(
                ui,
                node,
                path,
                actions,
                input,
                focus,
                focus_request,
                Some(size),
            );
            ui.add_space(6.0);
        }
        "paragraph" => {
            render_text_block(ui, node, path, actions, input, focus, focus_request, None);
            ui.add_space(4.0);
        }
        "codeBlock" => {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                render_text_block(
                    ui,
                    node,
                    path,
                    actions,
                    input,
                    focus,
                    focus_request,
                    Some(14.0),
                );
            });
            ui.add_space(6.0);
        }
        "bulletList" | "orderedList" | "taskList" => {
            render_list(ui, node, depth, path, actions, input, focus, focus_request);
        }
        "blockquote" => {
            ui.horizontal(|ui| {
                ui.add_space(8.0);
                ui.vertical(|ui| {
                    for (index, child) in node.children().iter().enumerate() {
                        path.push(index);
                        render_block(
                            ui,
                            child,
                            depth + 1,
                            path,
                            actions,
                            input,
                            focus,
                            focus_request,
                        );
                        path.pop();
                    }
                });
            });
        }
        "horizontalRule" => {
            ui.separator();
        }
        "table" => render_table(ui, node, depth, path, actions, input, focus, focus_request),
        "image" => {
            let alt = attr_str(node, "alt").unwrap_or("image");
            ui.label(format!("🖼 {alt}"));
        }
        "listItem" | "taskItem" | "tableRow" | "tableCell" | "tableHeader" => {
            for (index, child) in node.children().iter().enumerate() {
                path.push(index);
                render_block(ui, child, depth, path, actions, input, focus, focus_request);
                path.pop();
            }
        }
        other => {
            ui.label(egui::RichText::new(format!("[{other}]")).weak());
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_text_block(
    ui: &mut egui::Ui,
    node: &ProseMirrorNode,
    path: &[usize],
    actions: &mut Vec<EditorAction>,
    input: &ViewInput,
    focus: &mut ViewFocus,
    focus_request: &mut Option<NodePath>,
    size: Option<f32>,
) {
    let mut buffer = node.inline_text();
    let id = ui.make_persistent_id(("doc-text", path.to_vec()));
    let monospace = node.node_type == "codeBlock";
    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
        let job = text_layout(ui, node, text.as_str(), wrap_width, size, monospace);
        ui.fonts_mut(|fonts| fonts.layout_job(job))
    };
    let output = egui::TextEdit::multiline(&mut buffer)
        .id(id)
        .desired_width(f32::INFINITY)
        .frame(egui::Frame::NONE)
        .desired_rows(1)
        .return_key(None)
        // Keep Tab inside the editor so the list indentation command can use
        // it instead of egui moving keyboard focus to another widget.
        .event_filter(egui::EventFilter {
            tab: true,
            ..Default::default()
        })
        .layouter(&mut layouter)
        .show(ui);
    if focus_request.as_deref() == Some(path) {
        output.response.response.request_focus();
        *focus_request = None;
    }
    let response = output.response.response;
    // `TextEdit::multiline` turns Tab into a literal tab before we see it.
    // Suppress that edit and let the list indentation command win.
    let list_tab = (input.indent || input.outdent)
        && response.has_focus()
        && path.len() >= 3
        && path.len() % 2 == 1;
    if response.changed() && !list_tab {
        actions.push(EditorAction::Replace {
            path: path.to_vec(),
            text: buffer,
        });
    }
    if response.has_focus() {
        if let Some(range) = output.cursor_range {
            let start = range.primary.index.0.min(range.secondary.index.0);
            let end = range.primary.index.0.max(range.secondary.index.0);
            focus.path = Some(path.to_vec());
            focus.range = Some((start, end));
            if list_tab {
                if input.indent {
                    actions.push(EditorAction::Indent {
                        path: path.to_vec(),
                    });
                } else {
                    actions.push(EditorAction::Outdent {
                        path: path.to_vec(),
                    });
                }
            } else {
                if input.enter {
                    actions.push(EditorAction::Split {
                        path: path.to_vec(),
                        offset: range.primary.index.0,
                    });
                }
                if input.backspace && start == 0 && end == 0 {
                    actions.push(EditorAction::MergeBackward {
                        path: path.to_vec(),
                    });
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_list(
    ui: &mut egui::Ui,
    node: &ProseMirrorNode,
    depth: usize,
    path: &mut Vec<usize>,
    actions: &mut Vec<EditorAction>,
    input: &ViewInput,
    focus: &mut ViewFocus,
    focus_request: &mut Option<NodePath>,
) {
    let ordered = node.node_type == "orderedList";
    let tasks = node.node_type == "taskList";
    let start = attr_i64(node, "start").unwrap_or(1);
    for (index, item) in node.children().iter().enumerate() {
        path.push(index);
        ui.horizontal(|ui| {
            ui.add_space(depth as f32 * 16.0);
            if tasks {
                let checked = attr_bool(item, "checked").unwrap_or(false);
                if ui
                    .add(egui::Button::new(if checked { "☑" } else { "☐" }).frame(false))
                    .clicked()
                {
                    actions.push(EditorAction::ToggleTask { path: path.clone() });
                }
            } else if ordered {
                ui.label(format!("{}.", start + index as i64));
            } else {
                ui.label("•");
            }
            ui.vertical(|ui| {
                for (child_index, child) in item.children().iter().enumerate() {
                    path.push(child_index);
                    render_block(
                        ui,
                        child,
                        depth + 1,
                        path,
                        actions,
                        input,
                        focus,
                        focus_request,
                    );
                    path.pop();
                }
            });
        });
        path.pop();
    }
    ui.add_space(6.0);
}

#[allow(clippy::too_many_arguments)]
fn render_table(
    ui: &mut egui::Ui,
    node: &ProseMirrorNode,
    depth: usize,
    path: &mut Vec<usize>,
    actions: &mut Vec<EditorAction>,
    input: &ViewInput,
    focus: &mut ViewFocus,
    focus_request: &mut Option<NodePath>,
) {
    ui.push_id(path.clone(), |ui| {
        egui::Grid::new("table")
            .striped(true)
            .min_col_width(80.0)
            .show(ui, |ui| {
                for (row_index, row) in node.children().iter().enumerate() {
                    path.push(row_index);
                    for (cell_index, cell) in row.children().iter().enumerate() {
                        path.push(cell_index);
                        ui.vertical(|ui| {
                            for (child_index, child) in cell.children().iter().enumerate() {
                                path.push(child_index);
                                render_block(
                                    ui,
                                    child,
                                    depth + 1,
                                    path,
                                    actions,
                                    input,
                                    focus,
                                    focus_request,
                                );
                                path.pop();
                            }
                        });
                        path.pop();
                    }
                    path.pop();
                    ui.end_row();
                }
            });
    });
    ui.add_space(6.0);
}

fn text_layout(
    ui: &egui::Ui,
    node: &ProseMirrorNode,
    text: &str,
    wrap_width: f32,
    size: Option<f32>,
    monospace: bool,
) -> egui::text::LayoutJob {
    let body = egui::TextStyle::Body.resolve(ui.style());
    let base_size = size.unwrap_or(body.size);
    let base_font = if monospace {
        egui::FontId::monospace(base_size)
    } else {
        egui::FontId::proportional(base_size)
    };
    let base_color = if size.is_some() {
        ui.visuals().strong_text_color()
    } else {
        ui.visuals().text_color()
    };
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = wrap_width;

    if text != node.inline_text() {
        job.append(
            text,
            0.0,
            egui::TextFormat {
                font_id: base_font,
                color: base_color,
                ..Default::default()
            },
        );
        return job;
    }

    for child in node.children() {
        if let Some(run) = &child.text {
            let mut format = egui::TextFormat {
                font_id: base_font.clone(),
                color: base_color,
                ..Default::default()
            };
            if let Some(marks) = &child.marks {
                for mark in marks {
                    match mark.mark_type.as_str() {
                        "bold" => format.color = ui.visuals().strong_text_color(),
                        "italic" => format.italics = true,
                        "strike" => {
                            format.strikethrough = egui::Stroke::new(1.0, format.color);
                        }
                        "code" => {
                            format.font_id = egui::FontId::monospace(base_size);
                            format.background = ui.visuals().code_bg_color;
                        }
                        "link" => {
                            format.color = ui.visuals().hyperlink_color;
                            format.underline = egui::Stroke::new(1.0, format.color);
                        }
                        _ => {}
                    }
                }
            }
            job.append(run, 0.0, format);
        } else if child.node_type == "hardBreak" {
            job.append(
                "\n",
                0.0,
                egui::TextFormat {
                    font_id: base_font.clone(),
                    color: base_color,
                    ..Default::default()
                },
            );
        } else if child.node_type == "image" {
            job.append(
                "\u{FFFC}",
                0.0,
                egui::TextFormat {
                    font_id: base_font.clone(),
                    color: base_color,
                    ..Default::default()
                },
            );
        }
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;

    const CANONICAL_FIXTURE: &str =
        include_str!("../../../core/core-db/tests/fixtures/block-schema-v1.json");

    #[test]
    fn replace_text_keeps_marks_and_folds_newlines() {
        let doc = ProseMirrorDoc::from_json_str(CANONICAL_FIXTURE).expect("fixture parses");
        let mut editor = Editor::new(doc);
        let path = vec![1];
        let old = editor.doc().node(&path).expect("paragraph").inline_text();
        let bold_start = old.find("bold").expect("bold word");
        let next = old.replace("bold", "BOLD");
        assert!(replace_text(&mut editor, &path, &next));
        let node = editor.doc().node(&path).expect("paragraph");
        let bold = node
            .children()
            .iter()
            .find(|child| child.text.as_deref() == Some("BOLD"))
            .expect("replacement keeps the bold run");
        assert!(bold
            .marks
            .as_ref()
            .is_some_and(|marks| marks.iter().any(|mark| mark.mark_type == "bold")));
        assert!(bold_start > 0);

        assert!(replace_text(&mut editor, &path, "line\nbreak"));
        assert!(!editor
            .doc()
            .node(&path)
            .expect("paragraph")
            .inline_text()
            .contains('\n'));
    }

    #[test]
    fn renders_the_canonical_fixture_headlessly() {
        let doc = ProseMirrorDoc::from_json_str(CANONICAL_FIXTURE).expect("fixture parses");
        let context = egui::Context::default();
        let input = ViewInput::default();
        let mut actions = Vec::new();
        let mut focus = ViewFocus::default();
        let mut focus_request = None;
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            let mut path = Vec::new();
            for (index, node) in doc.content.iter().enumerate() {
                path.push(index);
                render_block(
                    ui,
                    node,
                    0,
                    &mut path,
                    &mut actions,
                    &input,
                    &mut focus,
                    &mut focus_request,
                );
                path.pop();
            }
        });
        output.textures_delta.clear();
    }
}
