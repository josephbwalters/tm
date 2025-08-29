use anyhow::Result;
use eframe::{
    egui::{self, Event, Key, Modifiers, RichText, ScrollArea},
    NativeOptions,
};
use slug::slugify;
use tm_core::{load_keymap_from_user, Action, Keymap, Status, Vault};

pub fn run_gui(vault: Vault) -> Result<()> {
    let native_options = NativeOptions::default();
    eframe::run_native(
        "tm — GUI",
        native_options,
        Box::new(move |_cc| {
            Box::new(App {
                vault,
                selected: 0,
                filter: String::new(),
                last_key_g: false,
                project_filter: None,
                new_project_title: String::new(),
                focus_new_project: false,
                keymap: load_keymap_from_user(),
            })
        }),
    )
    .map_err(|e| anyhow::Error::msg(e.to_string()))?;
    Ok(())
}

struct App {
    vault: Vault,
    selected: usize,
    filter: String,
    last_key_g: bool, // for 'gg'
    project_filter: Option<String>,
    new_project_title: String,
    focus_new_project: bool,
    keymap: Keymap,
}

fn egui_key_to_token(key: Key, mods: Modifiers) -> Option<String> {
    // Normalize to the same tokens as TUI keymap: "j", "k", "Ctrl-d", "G", "/", "1", etc.
    use Key::*;
    let ctrl = mods.ctrl;
    let shift = mods.shift;

    let base = match key {
        ArrowDown => "Down".to_string(),
        ArrowUp => "Up".to_string(),
        End => "End".to_string(),
        G => "g".to_string(),
        J => "j".to_string(),
        K => "k".to_string(),
        D => "d".to_string(),
        U => "u".to_string(),
        X => "x".to_string(),
        Num1 => "1".to_string(),
        Num2 => "2".to_string(),
        Num3 => "3".to_string(),
        Slash => "/".to_string(),
        Q => "q".to_string(),
        _ => return None,
    };

    Some(if ctrl {
        format!("Ctrl-{}", base)
    } else if shift && base.len() == 1 {
        base.to_uppercase()
    } else {
        base
    })
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let tasks = self.vault.list_tasks(None).unwrap_or_default();
        let len = tasks.len();

        // --- key handling (global) ---
        let input_snapshot = ctx.input(|i| i.clone());
        let mut action: Option<Action> = None;

        // support gg and G (use key_pressed)
        if input_snapshot.key_pressed(Key::G) {
            if input_snapshot.modifiers.shift {
                action = Some(Action::GoBottom); // Shift+G
            } else if self.last_key_g {
                action = Some(Action::GoTop); // gg
                self.last_key_g = false;
            } else {
                self.last_key_g = true;
            }
        } else {
            self.last_key_g = false;

            // Find the first Key press event this frame and feed to keymap
            let mut first_key: Option<(Key, Modifiers)> = None;
            for ev in &input_snapshot.events {
                if let Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = ev
                {
                    first_key = Some((*key, *modifiers));
                    break;
                }
            }
            if let Some((k, mods)) = first_key {
                if let Some(tok) = egui_key_to_token(k, mods) {
                    action = self.keymap.lookup(&tok);
                }
            }
        }

        // Extra GUI-only shortcuts
        if input_snapshot.key_pressed(Key::P) && input_snapshot.modifiers.shift {
            // focus "New project" field
            self.focus_new_project = true;
        }

        if let Some(act) = action {
            match act {
                // nav cases...
                Action::MoveDown => {
                    if self.selected + 1 < len {
                        self.selected += 1;
                    }
                }
                Action::MoveUp => {
                    if self.selected > 0 {
                        self.selected -= 1;
                    }
                }
                Action::HalfPageDown => {
                    let jump = (len.max(1) / 2).max(1);
                    self.selected = (self.selected + jump).min(len.saturating_sub(1));
                }
                Action::HalfPageUp => {
                    let jump = (len.max(1) / 2).max(1);
                    self.selected = self.selected.saturating_sub(jump);
                }
                Action::GoTop => self.selected = 0,
                Action::GoBottom => {
                    if len > 0 {
                        self.selected = len - 1;
                    }
                }
                Action::FocusFilter => { /* handled by focusing the filter input below */ }
                Action::Quit => { /* GUI ignores */ }

                Action::StatusNext | Action::StatusPrev | Action::SetTodo | Action::SetDoing | Action::SetDone => {
                    if let Some(t) = tasks.get(self.selected) {
                        let id = &t.id;
                        let _: anyhow::Result<Status> = match act {
                            Action::StatusNext => self.vault.cycle_status(id, 1),
                            Action::StatusPrev => self.vault.cycle_status(id, -1),
                            Action::SetTodo => self.vault.set_status(id, Status::Todo).map(|_| Status::Todo),
                            Action::SetDoing => self.vault.set_status(id, Status::Doing).map(|_| Status::Doing),
                            Action::SetDone => self.vault.set_status(id, Status::Done).map(|_| Status::Done),
                            _ => unreachable!(),
                        };
                    }
                }
            }
        }

        // --------- UI LAYOUT ---------
        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("tm").strong());
                ui.separator();

                // Project dropdown
                let mut keys: Vec<String> = self
                    .vault
                    .list_projects()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| p.key)
                    .collect();
                keys.sort();

                egui::ComboBox::from_label("Project")
                    .selected_text(self.project_filter.clone().unwrap_or_else(|| "(all)".into()))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.project_filter, None, "(all)".to_string());
                        for k in &keys {
                            ui.selectable_value(&mut self.project_filter, Some(k.clone()), k);
                        }
                    });

                ui.separator();
                ui.label("New:");
                let new_proj_widget =
                    egui::TextEdit::singleline(&mut self.new_project_title).id_source("new_proj_input");
                let resp = ui.add(new_proj_widget);
                if self.focus_new_project {
                    resp.request_focus();
                    self.focus_new_project = false;
                }

                let create_clicked = ui.button("Create").clicked();
                let enter_on_field = ui.input(|i| i.key_pressed(Key::Enter)) && ui.memory(|m| m.has_focus(resp.id));

                if create_clicked || enter_on_field {
                    let title = self.new_project_title.trim();
                    if !title.is_empty() {
                        let _ = self.vault.create_project(tm_core::ProjectNew {
                            title: title.to_string(),
                            tags: vec![],
                        });
                        self.project_filter = Some(slugify(title));
                        self.new_project_title.clear();
                    }
                }

                ui.separator();
                ui.label("Filter:");
                let filter_widget = egui::TextEdit::singleline(&mut self.filter).id_source("filter_input");
                let resp_filter = ui.add(filter_widget);
                // If user pressed key bound to FocusFilter this frame, focus the filter input
                if matches!(action, Some(Action::FocusFilter)) {
                    resp_filter.request_focus();
                }
            });
        });

        egui::SidePanel::left("left").resizable(true).default_width(420.0).show(ctx, |ui| {
            ui.heading("Tasks");
            ui.separator();
            ScrollArea::vertical().show(ui, |ui| {
                for (i, t) in tasks.iter().enumerate() {
                    // project filter + text filter
                    if let Some(pk) = &self.project_filter {
                        if &t.project != pk {
                            continue;
                        }
                    }
                    if !self.filter.is_empty() {
                        let hay = format!("[{}] {} {}", t.status, t.title, t.project).to_lowercase();
                        if !hay.contains(&self.filter.to_lowercase()) {
                            continue;
                        }
                    }

                    let selected = i == self.selected;
                    let text = format!("[{}] {}  · {}", t.status, t.title, t.project);
                    if ui.selectable_label(selected, text).clicked() {
                        self.selected = i;
                    }
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Detail");
            ui.separator();
            if tasks.is_empty() {
                ui.label("No tasks yet. Use the Add button or CLI.");
            } else {
                let idx = self.selected.min(tasks.len().saturating_sub(1));
                let t = &tasks[idx];
                ui.monospace(format!("id:      {}", t.id));
                ui.monospace(format!("title:   {}", t.title));
                ui.monospace(format!("status:  {}", t.status));
                ui.monospace(format!("project: {}", t.project));
                ui.monospace(format!("updated: {}", t.updated));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Todo (1)").clicked() {
                        let _ = self.vault.set_status(&t.id, Status::Todo);
                    }
                    if ui.button("In-Progress (2)").clicked() {
                        let _ = self.vault.set_status(&t.id, Status::Doing);
                    }
                    if ui.button("Done (3)").clicked() {
                        let _ = self.vault.set_status(&t.id, Status::Done);
                    }
                });
            }
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.label("Navigation: j/k, gg/G, Ctrl-d/u");
                ui.label("Filter/Projects: / focus filter · Project dropdown · Shift+P focus 'New project'");
                ui.label("Status: x next · X prev · 1/2/3 set todo/doing/done");
                ui.label("Edits: inline fields in the Detail panel (title/due/tags)");
                ui.label("Config: ~/.config/tm/config.lua (Lua keymaps); restart to reload");
            });
        });
    }
}

