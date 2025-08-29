use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{prelude::*, widgets::*};
use slug::slugify;
use tm_core::{
    load_keymap_from_user, parse_ex, Action, ExCommand, Keymap, Status, StatusSet, Vault,
};

fn keyevent_to_token(ev: KeyEvent) -> Option<String> {
    use KeyCode::*;
    let m = ev.modifiers;

    match ev.code {
        Char(c) => {
            if m.contains(KeyModifiers::CONTROL) {
                Some(format!("Ctrl-{}", c.to_ascii_lowercase()))
            } else if m.contains(KeyModifiers::SHIFT) && c.is_ascii_alphabetic() {
                Some(c.to_ascii_uppercase().to_string())
            } else {
                Some(c.to_string())
            }
        }
        Down => Some("Down".into()),
        Up => Some("Up".into()),
        Left => Some("Left".into()),
        Right => Some("Right".into()),
        End => Some("End".into()),
        Esc => Some("Esc".into()),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputMode {
    None,
    Filter,
    EditDue,
    EditTitle,
    EditTags,
    PickProject,
    NewProject,
}

pub fn run_tui(vault: Vault) -> Result<()> {
    // Load keymap from ~/.config/tm/config.lua (fallback to defaults)
    let mut keymap: Keymap = load_keymap_from_user();

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(&mut stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = ratatui::Terminal::new(backend)?;

    let mut selected: usize = 0; // index in filtered list
    let mut state = ListState::default();
    let mut last_key: Option<KeyCode> = None;

    // Filters & inputs
    let mut filter = String::new();
    let mut input_mode = InputMode::None;
    let mut input_buf = String::new();

    // EX command bar
    let mut ex_mode = false;
    let mut ex_input = String::new();
    // Result area (displayed even after ex-mode closes). is_error=false => green, true => red.
    let mut ex_result: Option<(bool, String)> = None;

    // Projects
    let mut projects: Vec<String> = vault
        .list_projects()
        .unwrap_or_default()
        .into_iter()
        .map(|p| p.key)
        .collect();
    projects.sort();
    let mut cur_project: Option<String> = None;
    let mut project_pick_idx: usize = 0;

    loop {
        let tasks_all = vault.list_tasks(None).unwrap_or_default();

        // Visible map: by project and text filter
        let matches_filter = |s: &str, t: &tm_core::Task| {
            if s.is_empty() {
                return true;
            }
            let hay = format!("[{}] {} {}", t.status, t.title, t.project).to_lowercase();
            hay.contains(&s.to_lowercase())
        };
        let visible: Vec<usize> = tasks_all
            .iter()
            .enumerate()
            .filter(|(_, t)| {
                cur_project
                    .as_ref()
                    .map(|pk| t.project == *pk)
                    .unwrap_or(true)
                    && matches_filter(&filter, t)
            })
            .map(|(i, _)| i)
            .collect();

        let len = visible.len();
        if len == 0 {
            selected = 0;
        } else if selected >= len {
            selected = len - 1;
        }
        state.select(Some(selected));

        // ---------- Draw ----------
        terminal.draw(|f| {
            let area = f.area();
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(1), Constraint::Min(1), Constraint::Length(3)])
                .split(area);

            // Header
            let hdr = match &cur_project {
                Some(p) => format!("Project: {p}   (O pick · ]/[ cycle · P new · / filter · : ex)"),
                None => "Project: (all)   (O pick · ]/[ cycle · P new · / filter · : ex)".to_string(),
            };
            let header = Paragraph::new(hdr).block(Block::default().borders(Borders::ALL));
            f.render_widget(header, rows[0]);

            // Main columns
            let cols = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                .split(rows[1]);

            // Left: tasks
            let items: Vec<ListItem> = visible
                .iter()
                .map(|&idx| {
                    let t = &tasks_all[idx];
                    ListItem::new(format!("[{}] {}  · {}", t.status, t.title, t.project))
                })
                .collect();
            let list = List::new(items)
                .highlight_symbol("➤ ")
                .block(Block::default().borders(Borders::ALL).title("Tasks"));
            f.render_stateful_widget(list, cols[0], &mut state);

            // Right: HELP (multiline)
            let help_text = vec![
                "Navigation:",
                "  j/k, gg/G, Ctrl-d/u, q (quit)",
                "",
                "Filtering & Projects:",
                "  / filter · O pick project · ]/[ next/prev project · P new project",
                "",
                "Status:",
                "  x next · X prev · 1 todo · 2 doing · 3 done",
                "",
                "Edits:",
                "  D due · R rename · T tags",
                "",
                "Ex commands:",
                "  :new \"Title\" project:<slug> +tag due:YYYY-MM-DD",
                "  :status [<id>] (todo|doing|done|next|prev)",
                "  :open project:<slug>",
                "  :project.new \"Title\" +tag",
                "  :config.reload",
                "",
                "Config:",
                "  ~/.config/tm/config.lua (Lua keymaps) — use :config.reload",
            ]
            .join("\n");

            let right = Paragraph::new(help_text)
                .block(Block::default().borders(Borders::ALL).title("Help"))
                .wrap(Wrap { trim: false });
            f.render_widget(right, cols[1]);

            // Bottom: ex bar (if active) OR other inputs
            if ex_mode {
                let p = Paragraph::new(format!(":{}", ex_input))
                    .block(Block::default().borders(Borders::ALL).title("command"));
                f.render_widget(p, rows[2]);
            } else {
                // If we have a recent result, show it here (colored).
                if let Some((is_err, msg)) = &ex_result {
                    let style = if *is_err {
                        Style::default().fg(Color::Red)
                    } else {
                        Style::default().fg(Color::Green)
                    };
                    let bottom = Paragraph::new(Span::styled(msg.clone(), style))
                        .block(Block::default().borders(Borders::ALL).title("result"));
                    f.render_widget(bottom, rows[2]);
                } else {
                    match input_mode {
                        InputMode::PickProject => {
                            let items: Vec<ListItem> =
                                projects.iter().map(|k| ListItem::new(k.to_string())).collect();
                            let mut st = ListState::default();
                            st.select(Some(project_pick_idx.min(projects.len().saturating_sub(1))));
                            let list = List::new(items)
                                .highlight_symbol("➤ ")
                                .block(
                                    Block::default()
                                        .borders(Borders::ALL)
                                        .title("Pick project (↑/↓, Enter, Esc)"),
                                );
                            f.render_stateful_widget(list, rows[2], &mut st);
                        }
                        InputMode::Filter => {
                            let bottom = Paragraph::new(format!("/{}", filter))
                                .block(Block::default().borders(Borders::ALL).title("Filter"));
                            f.render_widget(bottom, rows[2]);
                        }
                        InputMode::EditDue => {
                            let bottom = Paragraph::new(format!("due> {}", input_buf))
                                .block(
                                    Block::default()
                                        .borders(Borders::ALL)
                                        .title("Set Due (Enter/Esc)"),
                                );
                            f.render_widget(bottom, rows[2]);
                        }
                        InputMode::EditTitle => {
                            let bottom = Paragraph::new(format!("title> {}", input_buf))
                                .block(
                                    Block::default()
                                        .borders(Borders::ALL)
                                        .title("Rename (Enter/Esc)"),
                                );
                            f.render_widget(bottom, rows[2]);
                        }
                        InputMode::EditTags => {
                            let bottom = Paragraph::new(format!("tags> {}", input_buf))
                                .block(
                                    Block::default()
                                        .borders(Borders::ALL)
                                        .title("Set Tags (Enter/Esc)"),
                                );
                            f.render_widget(bottom, rows[2]);
                        }
                        InputMode::NewProject => {
                            let bottom = Paragraph::new(format!("project> {}", input_buf))
                                .block(
                                    Block::default()
                                        .borders(Borders::ALL)
                                        .title("New Project Title (Enter/Esc)"),
                                );
                            f.render_widget(bottom, rows[2]);
                        }
                        InputMode::None => {
                            let bottom = Paragraph::new("")
                                .block(Block::default().borders(Borders::ALL).title("Command"));
                            f.render_widget(bottom, rows[2]);
                        }
                    }
                }
            }
        })?;

        // ---------- Input ----------
        if event::poll(std::time::Duration::from_millis(120))? {
            let ev = event::read()?;
            if let Event::Key(k) = ev {
                // EX MODE takes priority
                if ex_mode {
                    match k.code {
                        KeyCode::Esc => {
                            ex_mode = false;
                            ex_input.clear();
                            // keep last result displayed
                        }
                        KeyCode::Enter => {
                            let line = ex_input.trim().trim_start_matches(':').to_string();
                            ex_input.clear();
                            ex_mode = false;

                            // Run + display result
                            match parse_ex(&line) {
                                Ok(cmd) => {
                                    let res_msg = match cmd {
                                        ExCommand::ConfigReload => {
                                            keymap = load_keymap_from_user();
                                            "config reloaded".to_string()
                                        }
                                        ExCommand::New { title, project, tags, due } => {
                                            let proj = project.unwrap_or_else(|| "inbox".into());
                                            match vault.create_task(tm_core::TaskNew {
                                                title: title.clone(),
                                                project: proj.clone(),
                                                due,
                                                tags,
                                            }) {
                                                Ok(id) => format!("created task {id} in project {proj}"),
                                                Err(e) => {
                                                    ex_result = Some((true, e.to_string()));
                                                    continue;
                                                }
                                            }
                                        }
                                        ExCommand::Status { id, set } => {
                                            // Use provided id or current selection
                                            let use_id = id.or_else(|| {
                                                visible
                                                    .get(selected)
                                                    .map(|&i| tasks_all[i].id.clone())
                                            });
                                            if let Some(id) = use_id {
                                                let msg = match set {
                                                    StatusSet::Todo => {
                                                        vault.set_status(&id, Status::Todo)
                                                            .map(|_| "status set: todo".to_string())
                                                    }
                                                    StatusSet::Doing => {
                                                        vault.set_status(&id, Status::Doing)
                                                            .map(|_| "status set: doing".to_string())
                                                    }
                                                    StatusSet::Done => {
                                                        vault.set_status(&id, Status::Done)
                                                            .map(|_| "status set: done".to_string())
                                                    }
                                                    StatusSet::Next => {
                                                        vault.cycle_status(&id, 1)
                                                            .map(|s| format!("status -> {}", s.as_str()))
                                                    }
                                                    StatusSet::Prev => {
                                                        vault.cycle_status(&id, -1)
                                                            .map(|s| format!("status -> {}", s.as_str()))
                                                    }
                                                };
                                                match msg {
                                                    Ok(m) => m,
                                                    Err(e) => {
                                                        ex_result = Some((true, e.to_string()));
                                                        continue;
                                                    }
                                                }
                                            } else {
                                                ex_result =
                                                    Some((true, "no task selected".into()));
                                                continue;
                                            }
                                        }
                                        ExCommand::OpenProject { key } => {
                                            if key.is_empty() {
                                                cur_project = None;
                                                "opened all projects".into()
                                            } else {
                                                cur_project = Some(key.clone());
                                                selected = 0;
                                                format!("opened project {key}")
                                            }
                                        }
                                        ExCommand::ProjectNew { title, tags } => {
                                            match vault.create_project(tm_core::ProjectNew {
                                                title: title.clone(),
                                                tags,
                                            }) {
                                                Ok(k) => {
                                                    // refresh projects + jump into it
                                                    projects = vault
                                                        .list_projects()
                                                        .unwrap_or_default()
                                                        .into_iter()
                                                        .map(|p| p.key)
                                                        .collect();
                                                    projects.sort();
                                                    cur_project = Some(slugify(&title));
                                                    selected = 0;
                                                    format!("created project {k}")
                                                }
                                                Err(e) => {
                                                    ex_result = Some((true, e.to_string()));
                                                    continue;
                                                }
                                            }
                                        }
                                    };
                                    ex_result = Some((false, res_msg));
                                }
                                Err(e) => {
                                    ex_result = Some((true, e.to_string()));
                                }
                            }
                        }
                        KeyCode::Backspace => {
                            ex_input.pop();
                        }
                        KeyCode::Char(c) => {
                            if !k.modifiers.contains(KeyModifiers::CONTROL) {
                                ex_input.push(c);
                            }
                        }
                        _ => {}
                    }
                    continue; // don't process other modes while ex is active
                }

                // Mode-specific input first
                match input_mode {
                    InputMode::Filter => {
                        match k.code {
                            KeyCode::Esc | KeyCode::Enter => input_mode = InputMode::None,
                            KeyCode::Backspace => {
                                filter.pop();
                            }
                            KeyCode::Char(c) => {
                                if !k.modifiers.contains(KeyModifiers::CONTROL) {
                                    filter.push(c);
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }
                    InputMode::EditDue | InputMode::EditTitle | InputMode::EditTags => {
                        match k.code {
                            KeyCode::Esc => {
                                input_mode = InputMode::None;
                                input_buf.clear();
                            }
                            KeyCode::Enter => {
                                if let Some(&orig_idx) = visible.get(selected) {
                                    let id = &tasks_all[orig_idx].id;
                                    let res = match input_mode {
                                        InputMode::EditDue => vault.set_due(id, &input_buf),
                                        InputMode::EditTitle => vault.rename_title(id, &input_buf),
                                        InputMode::EditTags => vault.set_tags_csv(id, &input_buf),
                                        _ => Ok(()),
                                    };
                                    ex_result = Some(match res {
                                        Ok(_) => (false, "saved".into()),
                                        Err(e) => (true, e.to_string()),
                                    });
                                }
                                input_mode = InputMode::None;
                                input_buf.clear();
                            }
                            KeyCode::Backspace => {
                                input_buf.pop();
                            }
                            KeyCode::Char(c) => {
                                if !k.modifiers.contains(KeyModifiers::CONTROL) {
                                    input_buf.push(c);
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }
                    InputMode::PickProject => {
                        match k.code {
                            KeyCode::Esc => {
                                input_mode = InputMode::None;
                            }
                            KeyCode::Enter => {
                                if projects.is_empty() {
                                    cur_project = None;
                                } else {
                                    project_pick_idx = project_pick_idx.min(projects.len().saturating_sub(1));
                                    cur_project = Some(projects[project_pick_idx].clone());
                                    selected = 0;
                                }
                                input_mode = InputMode::None;
                            }
                            KeyCode::Up => {
                                project_pick_idx = project_pick_idx.saturating_sub(1);
                            }
                            KeyCode::Down => {
                                if project_pick_idx + 1 < projects.len() {
                                    project_pick_idx += 1;
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }
                    InputMode::NewProject => {
                        match k.code {
                            KeyCode::Esc => {
                                input_mode = InputMode::None;
                                input_buf.clear();
                            }
                            KeyCode::Enter => {
                                let title = input_buf.trim();
                                if !title.is_empty() {
                                    match vault.create_project(tm_core::ProjectNew {
                                        title: title.to_string(),
                                        tags: vec![],
                                    }) {
                                        Ok(k) => {
                                            projects = vault
                                                .list_projects()
                                                .unwrap_or_default()
                                                .into_iter()
                                                .map(|p| p.key)
                                                .collect();
                                            projects.sort();
                                            cur_project = Some(slugify(title));
                                            selected = 0;
                                            ex_result = Some((false, format!("created project {k}")));
                                        }
                                        Err(e) => {
                                            ex_result = Some((true, e.to_string()));
                                        }
                                    }
                                }
                                input_mode = InputMode::None;
                                input_buf.clear();
                            }
                            KeyCode::Backspace => {
                                input_buf.pop();
                            }
                            KeyCode::Char(c) => {
                                if !k.modifiers.contains(KeyModifiers::CONTROL) {
                                    input_buf.push(c);
                                }
                            }
                            _ => {}
                        }
                        continue;
                    }
                    InputMode::None => { /* fall through */ }
                }

                // Open ex bar with ':'
                if matches!(k.code, KeyCode::Char(':')) {
                    ex_mode = true;
                    ex_input.clear();
                    // keep last ex_result shown until replaced
                    continue;
                }

                // Global project actions not driven by keymap
                match (k.code, k.modifiers) {
                    (KeyCode::Char('O'), _) => {
                        projects = vault
                            .list_projects()
                            .unwrap_or_default()
                            .into_iter()
                            .map(|p| p.key)
                            .collect();
                        projects.sort();
                        project_pick_idx = 0;
                        input_mode = InputMode::PickProject;
                        continue;
                    }
                    (KeyCode::Char(']'), _) => {
                        if projects.is_empty() {
                            cur_project = None;
                        } else {
                            let idx = cur_project
                                .as_ref()
                                .and_then(|k| projects.iter().position(|p| p == k))
                                .map(|i| (i + 1) % projects.len())
                                .unwrap_or(0);
                            cur_project = Some(projects[idx].clone());
                            selected = 0;
                        }
                        continue;
                    }
                    (KeyCode::Char('['), _) => {
                        if projects.is_empty() {
                            cur_project = None;
                        } else {
                            let idx = cur_project
                                .as_ref()
                                .and_then(|k| projects.iter().position(|p| p == k))
                                .map(|i| if i == 0 { projects.len() - 1 } else { i - 1 })
                                .unwrap_or(0);
                            cur_project = Some(projects[idx].clone());
                            selected = 0;
                        }
                        continue;
                    }
                    (KeyCode::Char('P'), _) => {
                        input_mode = InputMode::NewProject;
                        input_buf.clear();
                        continue;
                    }
                    _ => {}
                }

                // ----- Keymap-driven single-key actions (plus gg sequence) -----
                let mut action: Option<Action> = None;

                // gg sequence (hardcoded for now)
                if let KeyCode::Char('g') = k.code {
                    if let Some(KeyCode::Char('g')) = last_key {
                        last_key = None;
                        action = Some(Action::GoTop);
                    } else {
                        last_key = Some(KeyCode::Char('g'));
                    }
                } else {
                    last_key = None;
                    if let Some(tok) = keyevent_to_token(k) {
                        action = keymap.lookup(&tok);
                    }
                }

                if let Some(act) = action {
                    match act {
                        Action::MoveDown => if len > 0 && selected + 1 < len { selected += 1; },
                        Action::MoveUp   => if selected > 0 { selected -= 1; },
                        Action::HalfPageDown => {
                            let jump = (len.max(1) / 2).max(1);
                            selected = (selected + jump).min(len.saturating_sub(1));
                        }
                        Action::HalfPageUp => {
                            let jump = (len.max(1) / 2).max(1);
                            selected = selected.saturating_sub(jump);
                        }
                        Action::GoTop    => selected = 0,
                        Action::GoBottom => if len > 0 { selected = len - 1; },
                        Action::FocusFilter => { input_mode = InputMode::Filter; }
                        Action::Quit => break,

                        Action::StatusNext | Action::StatusPrev | Action::SetTodo | Action::SetDoing | Action::SetDone => {
                            if let Some(&orig_idx) = visible.get(selected) {
                                let id = &tasks_all[orig_idx].id;
                                let res: anyhow::Result<Status> = match act {
                                    Action::StatusNext => vault.cycle_status(id, 1),
                                    Action::StatusPrev => vault.cycle_status(id, -1),
                                    Action::SetTodo    => vault.set_status(id, Status::Todo ).map(|_| Status::Todo ),
                                    Action::SetDoing   => vault.set_status(id, Status::Doing).map(|_| Status::Doing),
                                    Action::SetDone    => vault.set_status(id, Status::Done ).map(|_| Status::Done ),
                                    _ => unreachable!(),
                                };
                                ex_result = Some(match res {
                                    Ok(s) => (false, format!("status -> {}", s.as_str())),
                                    Err(e) => (true, e.to_string()),
                                });
                            }
                        }
                    }
                } else {
                    // Edit panels (hotkeys)
                    match (k.code, k.modifiers) {
                        (KeyCode::Char('D'), _) => { input_mode = InputMode::EditDue;   input_buf.clear(); }
                        (KeyCode::Char('R'), _) => { input_mode = InputMode::EditTitle; input_buf.clear(); }
                        (KeyCode::Char('T'), _) => { input_mode = InputMode::EditTags;  input_buf.clear(); }
                        (KeyCode::Char('/'), _) => { input_mode = InputMode::Filter; }
                        _ => {}
                    }
                }
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    Ok(())
}

