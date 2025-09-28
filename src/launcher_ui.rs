use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use anyhow::Context;
use gtk::glib::{self, clone};
use gtk::prelude::*;
use gtk4 as gtk;

use crate::apps::{Tool, ToolKind, ToolState};
use crate::bench_ops::{self, add_tool_to_bench};
use crate::model::Bench;
use crate::sway;

enum LauncherAction {
    Assemble,
    Stow,
    Sync,
    AddTool,
    AddWindow,
}

impl LauncherAction {
    fn label(&self, bench: &str) -> String {
        match self {
            LauncherAction::Assemble => format!("Launching {}", bench),
            LauncherAction::Stow => format!("Stowing {}", bench),
            LauncherAction::Sync => format!("Syncing {}", bench),
            LauncherAction::AddTool => format!("Adding tool to {}", bench),
            LauncherAction::AddWindow => format!("Adding window to {}", bench),
        }
    }

    fn success(&self, bench: &str) -> String {
        match self {
            LauncherAction::Assemble => format!("Launched {}", bench),
            LauncherAction::Stow => format!("Stowed {}", bench),
            LauncherAction::Sync => format!("Synced {}", bench),
            LauncherAction::AddTool => format!("Added tool to {}", bench),
            LauncherAction::AddWindow => format!("Added window to {}", bench),
        }
    }

    fn verb(&self) -> &'static str {
        match self {
            LauncherAction::Assemble => "launch",
            LauncherAction::Stow => "stow",
            LauncherAction::Sync => "sync",
            LauncherAction::AddTool => "add tool",
            LauncherAction::AddWindow => "add window",
        }
    }
}

pub fn run(benches_dir: PathBuf) -> anyhow::Result<()> {
    let benches_dir = Arc::new(benches_dir);
    let app = gtk::Application::new(Some("com.tyler.bench.launcher"), Default::default());

    app.connect_activate(clone!(@strong benches_dir => move |app| {
        if let Err(err) = build_ui(app, benches_dir.clone()) {
            show_error(app, &err.to_string());
        }
    }));

    app.run();
    Ok(())
}

fn build_ui(app: &gtk::Application, benches_dir: Arc<PathBuf>) -> anyhow::Result<()> {
    let window = gtk::ApplicationWindow::new(app);
    window.set_title(Some("Bench Launcher"));
    window.set_default_size(480, 360);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
    vbox.set_margin_all(12);

    let search_entry = gtk::Entry::new();
    search_entry.set_placeholder_text(Some("Search benches"));
    vbox.append(&search_entry);

    let scrolled = gtk::ScrolledWindow::new();
    scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    scrolled.set_vexpand(true);

    let list_box = gtk::ListBox::new();
    list_box.set_selection_mode(gtk::SelectionMode::Single);
    list_box.set_vexpand(true);
    list_box.set_activate_on_single_click(false);
    scrolled.set_child(Some(&list_box));
    vbox.append(&scrolled);

    let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let add_tool_btn = gtk::Button::with_label("Add Tool");
    let add_window_btn = gtk::Button::with_label("Add Window");
    button_box.append(&add_tool_btn);
    button_box.append(&add_window_btn);
    vbox.append(&button_box);

    let status_label = gtk::Label::new(None);
    status_label.set_xalign(0.0);
    status_label.set_ellipsize(pango::EllipsizeMode::End);
    vbox.append(&status_label);

    window.set_child(Some(&vbox));

    add_tool_btn.connect_clicked(clone!(@weak status_label, @strong benches_dir, @weak list_box => move |_| {
        if let Some(bench_name) = selected_bench_name(&list_box) {
            spawn_action(benches_dir.clone(), bench_name, LauncherAction::AddTool, status_label.clone());
        }
    }));

    add_window_btn.connect_clicked(clone!(@weak status_label, @strong benches_dir, @weak list_box => move |_| {
        if let Some(bench_name) = selected_bench_name(&list_box) {
            spawn_action(benches_dir.clone(), bench_name, LauncherAction::AddWindow, status_label.clone());
        }
    }));

    populate_list(&list_box, &benches_dir)?;

    connect_handlers(
        &window,
        &search_entry,
        &list_box,
        &status_label,
        &add_tool_btn,
        &add_window_btn,
        benches_dir.clone(),
    );

    window.present();
    Ok(())
}

fn populate_list(list_box: &gtk::ListBox, benches_dir: &PathBuf) -> anyhow::Result<()> {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }
    for bench in crate::bench_ops::list_benches(benches_dir)? {
        let row = gtk::ListBoxRow::new();
        row.set_focusable(true);
        let label = gtk::Label::new(Some(&bench));
        label.set_xalign(0.0);
        label.set_margin_all(6);
        row.set_child(Some(&label));
        list_box.append(&row);
    }
    if let Some(row) = list_box.row_at_index(0) {
        list_box.select_row(Some(&row));
        row.grab_focus();
    }
    Ok(())
}

fn connect_handlers(
    window: &gtk::ApplicationWindow,
    search_entry: &gtk::Entry,
    list_box: &gtk::ListBox,
    status_label: &gtk::Label,
    add_tool_btn: &gtk::Button,
    add_window_btn: &gtk::Button,
    benches_dir: Arc<PathBuf>,
) {
    search_entry.connect_changed(clone!(@weak list_box => move |entry| {
        let query = entry.text().to_string().to_lowercase();
        list_box.foreach(|widget| {
            if let Ok(row) = widget.clone().downcast::<gtk::ListBoxRow>() {
                if let Some(label) = row.child().and_then(|w| w.downcast::<gtk::Label>().ok()) {
                    let text = label.text().to_string();
                    let visible = query.is_empty() || text.to_lowercase().contains(&query);
                    row.set_visible(visible);
                }
            }
        });
    }));

    search_entry.connect_key_press_event(
        clone!(@weak list_box => @default-return gtk::Inhibit(false), move |entry, event| {
            match event.keyval() {
                gdk::Key::Down => {
                    if let Some(row) = list_box.row_at_index(0) {
                        list_box.select_row(Some(&row));
                        row.grab_focus();
                    }
                    gtk::Inhibit(true)
                }
                _ => gtk::Inhibit(false),
            }
        }),
    );

    list_box.connect_row_activated(clone!(@weak status_label, @strong benches_dir => move |_, row| {
        if let Some(label) = row.child().and_then(|w| w.downcast::<gtk::Label>().ok()) {
            let bench_name = label.text().to_string();
            spawn_action(benches_dir.clone(), bench_name, LauncherAction::Assemble, status_label.clone());
        }
    }));

    window.connect_key_press_event(clone!(@weak list_box, @weak status_label, @strong benches_dir => @default-return gtk::Inhibit(false), move |_, event| {
        let key = event.keyval();
        let modifier = event.state();
        let selected_row = list_box.selected_row();
        if let Some(row) = selected_row {
            if let Some(label) = row.child().and_then(|w| w.downcast::<gtk::Label>().ok()) {
                let bench_name = label.text().to_string();
                match (key, modifier.contains(gdk::ModifierType::SHIFT_MASK), modifier.contains(gdk::ModifierType::CONTROL_MASK), modifier.contains(gdk::ModifierType::ALT_MASK)) {
                    (gdk::Key::Return, false, false, _) => {
                        spawn_action(benches_dir.clone(), bench_name, LauncherAction::Assemble, status_label.clone());
                        return gtk::Inhibit(true);
                    }
                    (gdk::Key::Return, true, false, _) => {
                        spawn_action(benches_dir.clone(), bench_name, LauncherAction::Stow, status_label.clone());
                        return gtk::Inhibit(true);
                    }
                    (gdk::Key::S, false, true, _) => {
                        spawn_action(benches_dir.clone(), bench_name, LauncherAction::Sync, status_label.clone());
                        return gtk::Inhibit(true);
                    }
                    (gdk::Key::A, false, false, true) => {
                        spawn_action(benches_dir.clone(), bench_name, LauncherAction::AddTool, status_label.clone());
                        return gtk::Inhibit(true);
                    }
                    (gdk::Key::W, false, false, true) => {
                        spawn_action(benches_dir.clone(), bench_name, LauncherAction::AddWindow, status_label.clone());
                        return gtk::Inhibit(true);
                    }
                    _ => {}
                }
            }
        }
        if key == gdk::Key::Escape {
            window.close();
            return gtk::Inhibit(true);
        }
        gtk::Inhibit(false)
    }));
}

fn spawn_action(
    benches_dir: Arc<PathBuf>,
    bench_name: String,
    action: LauncherAction,
    status_label: gtk::Label,
) {
    status_label.set_text(&action.label(&bench_name));
    let (sender, receiver) = glib::MainContext::channel(glib::Priority::default());

    thread::spawn(move || {
        let result = run_action(benches_dir, &bench_name, &action)
            .map(|_| action.success(&bench_name))
            .unwrap_or_else(|err| format!("Failed to {} {}: {}", action.verb(), bench_name, err));
        let _ = sender.send(result);
    });

    receiver.attach(None, move |message| {
        status_label.set_text(&message);
        glib::Continue(false)
    });
}

fn run_action(
    benches_dir: Arc<PathBuf>,
    bench_name: &str,
    action: &LauncherAction,
) -> anyhow::Result<()> {
    let benches_dir_path = benches_dir.as_ref();
    let path = crate::bench_ops::resolve_bench_path(bench_name, benches_dir_path);
    let mut bench = crate::bench_ops::load_bench(&path)
        .with_context(|| format!("failed to load bench {}", bench_name))?;
    match action {
        LauncherAction::Assemble => crate::bench_ops::assemble_bench(&mut bench)?,
        LauncherAction::Stow => crate::bench_ops::stow_bench(&mut bench)?,
        LauncherAction::Sync => crate::bench_ops::snapshot_untracked_into_bench(&mut bench)?,
    }
    crate::bench_ops::save_bench(&path, &bench)?;
    Ok(())
}

fn show_error(app: &gtk::Application, message: &str) {
    let dialog = gtk::MessageDialog::builder()
        .transient_for(app.active_window().as_ref())
        .modal(true)
        .message_type(gtk::MessageType::Error)
        .buttons(gtk::ButtonsType::Close)
        .text(message)
        .build();
    dialog.connect_response(|dialog, _| dialog.close());
    dialog.present();
}

mod gdk {
    pub use gtk::gdk::Key;
    pub use gtk::gdk::ModifierType;
}

mod pango {
    pub use gtk::pango::EllipsizeMode;
}
