use std::path::PathBuf;
use std::sync::Arc;

use gtk::glib::{self, clone};
use gtk::prelude::*;
use gtk4 as gtk;

use crate::apps::ToolKind;
use crate::bench_ops;

pub fn run(benches_dir: PathBuf) -> anyhow::Result<()> {
    let benches_dir = Arc::new(benches_dir);
    let app = gtk::Application::new(
        Some("com.tyler.bench.launcher"),
        gtk::gio::ApplicationFlags::FLAGS_NONE,
    );

    app.connect_activate(clone!(@strong benches_dir => move |app| {
        if let Err(err) = build_ui(app, benches_dir.clone()) {
            show_error(app, &err.to_string());
        }
    }));

    app.run_with_args::<&str>(&[]);
    Ok(())
}

fn build_ui(app: &gtk::Application, benches_dir: Arc<PathBuf>) -> anyhow::Result<()> {
    let window = gtk::ApplicationWindow::new(app);
    window.set_title(Some("Bench Launcher"));
    window.set_default_size(480, 360);

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);

    let title_label = gtk::Label::new(Some("Bench Launcher"));
    title_label.set_markup("<span size='large' weight='bold'>Bench Launcher</span>");
    title_label.set_xalign(0.0);
    vbox.append(&title_label);

    let menu_box = gtk::Box::new(gtk::Orientation::Vertical, 4);
    menu_box.set_margin_top(12);

    let assemble_label = gtk::Label::new(Some("[a] Assemble - Add tool to focused bench"));
    assemble_label.set_xalign(0.0);
    assemble_label.set_margin_start(12);
    menu_box.append(&assemble_label);

    let focus_label = gtk::Label::new(Some("[f] Focus - Focus a bench"));
    focus_label.set_xalign(0.0);
    focus_label.set_margin_start(12);
    menu_box.append(&focus_label);

    let craft_label = gtk::Label::new(Some("[c] Craft - Create a new tool"));
    craft_label.set_xalign(0.0);
    craft_label.set_margin_start(12);
    menu_box.append(&craft_label);

    vbox.append(&menu_box);

    // This will be used to show different screens
    let content_stack = gtk::Stack::new();
    content_stack.set_vexpand(true);
    content_stack.set_margin_top(12);

    // Create the different mode screens
    let menu_screen = create_menu_screen();
    let assemble_screen = create_assemble_screen(benches_dir.clone())?;
    let focus_screen = create_focus_screen(benches_dir.clone())?;
    let craft_screen = create_craft_screen(benches_dir.clone())?;

    content_stack.add_named(&menu_screen, Some("menu"));
    content_stack.add_named(&assemble_screen, Some("assemble"));
    content_stack.add_named(&focus_screen, Some("focus"));
    content_stack.add_named(&craft_screen, Some("craft"));

    content_stack.set_visible_child_name("menu");
    vbox.append(&content_stack);

    let status_label = gtk::Label::new(None);
    status_label.set_xalign(0.0);
    status_label.set_ellipsize(pango::EllipsizeMode::End);
    vbox.append(&status_label);

    window.set_child(Some(&vbox));

    // Set up keyboard handlers
    let key_controller = gtk::EventControllerKey::new();
    key_controller.connect_key_pressed(clone!(@weak content_stack, @weak status_label, @weak window => @default-return glib::Propagation::Proceed, move |_, key, _, _| {
        match key {
            gdk::Key::Escape => {
                if content_stack.visible_child_name().as_deref() == Some("menu") {
                    window.close();
                } else {
                    content_stack.set_visible_child_name("menu");
                    status_label.set_text("");
                }
                return glib::Propagation::Stop;
            }
            gdk::Key::a => {
                content_stack.set_visible_child_name("assemble");
                return glib::Propagation::Stop;
            }
            gdk::Key::f => {
                content_stack.set_visible_child_name("focus");
                return glib::Propagation::Stop;
            }
            gdk::Key::c => {
                content_stack.set_visible_child_name("craft");
                return glib::Propagation::Stop;
            }
            _ => {}
        }
        glib::Propagation::Proceed
    }));
    window.add_controller(key_controller);

    window.present();
    Ok(())
}

fn create_menu_screen() -> gtk::Widget {
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);
    vbox.set_valign(gtk::Align::Center);

    let label = gtk::Label::new(Some("Press a key to select an option"));
    label.set_margin_start(20);
    label.set_margin_end(20);
    label.set_margin_top(20);
    label.set_margin_bottom(20);
    vbox.append(&label);

    vbox.upcast()
}

fn create_assemble_screen(benches_dir: Arc<PathBuf>) -> anyhow::Result<gtk::Widget> {
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);

    let header_label = gtk::Label::new(Some("Add Tool to Focused Bench"));
    header_label.set_markup("<span weight='bold'>Add Tool to Focused Bench</span>");
    header_label.set_xalign(0.0);
    vbox.append(&header_label);

    let search_entry = gtk::Entry::new();
    search_entry.set_placeholder_text(Some("Search tools"));
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

    // Populate with tools
    for tool in bench_ops::list_tools()? {
        let row = gtk::ListBoxRow::new();
        row.set_focusable(true);
        let label = gtk::Label::new(Some(&tool));
        label.set_xalign(0.0);
        label.set_margin_start(6);
        label.set_margin_end(6);
        label.set_margin_top(6);
        label.set_margin_bottom(6);
        row.set_child(Some(&label));
        list_box.append(&row);
    }

    if let Some(row) = list_box.row_at_index(0) {
        list_box.select_row(Some(&row));
        row.grab_focus();
    }

    // Handle search
    search_entry.connect_changed(clone!(@weak list_box => move |entry| {
        let query = entry.text().to_string().to_lowercase();
        let mut index = 0;
        while let Some(row) = list_box.row_at_index(index) {
            if let Some(label) = row.child().and_then(|w| w.downcast::<gtk::Label>().ok()) {
                let text = label.text().to_string();
                let visible = query.is_empty() || text.to_lowercase().contains(&query);
                row.set_visible(visible);
            }
            index += 1;
        }
    }));

    // Handle Enter key to add tool
    list_box.connect_row_activated(clone!(@strong benches_dir => move |_, row| {
        if let Some(label) = row.child().and_then(|w| w.downcast::<gtk::Label>().ok()) {
            let tool_name = label.text().to_string();
            if let Err(e) = add_tool_to_focused_bench(&tool_name) {
                eprintln!("Failed to add tool: {}", e);
            } else {
                println!("Added tool {} to focused bench", tool_name);
            }
        }
    }));

    Ok(vbox.upcast())
}

fn create_focus_screen(_benches_dir: Arc<PathBuf>) -> anyhow::Result<gtk::Widget> {
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);

    let header_label = gtk::Label::new(Some("Focus a Bench"));
    header_label.set_markup("<span weight='bold'>Focus a Bench</span>");
    header_label.set_xalign(0.0);
    vbox.append(&header_label);

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

    // Populate with benches
    for bench in bench_ops::list_benches()? {
        let row = gtk::ListBoxRow::new();
        row.set_focusable(true);
        let label = gtk::Label::new(Some(&bench));
        label.set_xalign(0.0);
        label.set_margin_start(6);
        label.set_margin_end(6);
        label.set_margin_top(6);
        label.set_margin_bottom(6);
        row.set_child(Some(&label));
        list_box.append(&row);
    }

    if let Some(row) = list_box.row_at_index(0) {
        list_box.select_row(Some(&row));
        row.grab_focus();
    }

    // Handle search
    search_entry.connect_changed(clone!(@weak list_box => move |entry| {
        let query = entry.text().to_string().to_lowercase();
        let mut index = 0;
        while let Some(row) = list_box.row_at_index(index) {
            if let Some(label) = row.child().and_then(|w| w.downcast::<gtk::Label>().ok()) {
                let text = label.text().to_string();
                let visible = query.is_empty() || text.to_lowercase().contains(&query);
                row.set_visible(visible);
            }
            index += 1;
        }
    }));

    // Handle Enter key to focus bench
    list_box.connect_row_activated(move |_, row| {
        if let Some(label) = row.child().and_then(|w| w.downcast::<gtk::Label>().ok()) {
            let bench_name = label.text().to_string();
            if let Err(e) = bench_ops::focus(&bench_name, true) {
                eprintln!("Failed to focus bench: {}", e);
            } else {
                println!("Focused bench {}", bench_name);
            }
        }
    });

    Ok(vbox.upcast())
}

fn create_craft_screen(_benches_dir: Arc<PathBuf>) -> anyhow::Result<gtk::Widget> {
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 8);

    let header_label = gtk::Label::new(Some("Craft a New Tool"));
    header_label.set_markup("<span weight='bold'>Craft a New Tool</span>");
    header_label.set_xalign(0.0);
    vbox.append(&header_label);

    let name_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let name_label = gtk::Label::new(Some("Name:"));
    name_label.set_width_chars(10);
    name_label.set_xalign(0.0);
    name_box.append(&name_label);

    let name_entry = gtk::Entry::new();
    name_entry.set_hexpand(true);
    name_entry.set_placeholder_text(Some("my-tool"));
    name_box.append(&name_entry);
    vbox.append(&name_box);

    let kind_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    kind_box.set_margin_top(8);
    let kind_label = gtk::Label::new(Some("Type:"));
    kind_label.set_width_chars(10);
    kind_label.set_xalign(0.0);
    kind_box.append(&kind_label);

    let kind_combo = gtk::ComboBoxText::new();
    kind_combo.append_text("Browser");
    kind_combo.append_text("Terminal");
    kind_combo.append_text("Zed");
    kind_combo.set_active(Some(0));
    kind_box.append(&kind_combo);
    vbox.append(&kind_box);

    let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    button_box.set_margin_top(12);
    button_box.set_halign(gtk::Align::End);

    let create_button = gtk::Button::with_label("Create Tool");
    create_button.add_css_class("suggested-action");
    button_box.append(&create_button);
    vbox.append(&button_box);

    let status_label = gtk::Label::new(None);
    status_label.set_xalign(0.0);
    status_label.set_margin_top(8);
    vbox.append(&status_label);

    // Handle create button
    create_button.connect_clicked(
        clone!(@weak name_entry, @weak kind_combo, @weak status_label => move |_| {
            let name = name_entry.text().to_string();
            if name.is_empty() {
                status_label.set_text("Error: Name cannot be empty");
                return;
            }

            let kind_str = kind_combo.active_text().unwrap_or_default();
            let kind = match kind_str.as_str() {
                "Browser" => ToolKind::Browser,
                "Terminal" => ToolKind::Terminal,
                "Zed" => ToolKind::Zed,
                _ => {
                    status_label.set_text("Error: Invalid tool type");
                    return;
                }
            };

            match bench_ops::craft_tool(kind, &name) {
                Ok(_) => {
                    status_label.set_text(&format!("✓ Created tool '{}'", name));
                    name_entry.set_text("");
                }
                Err(e) => {
                    status_label.set_text(&format!("Error: {}", e));
                }
            }
        }),
    );

    Ok(vbox.upcast())
}

fn add_tool_to_focused_bench(tool_name: &str) -> anyhow::Result<()> {
    let focused = bench_ops::focused_bench()?
        .ok_or_else(|| anyhow::anyhow!("No bench is currently focused"))?;

    // For now, default to adding to a bay named "default"
    // TODO: Could make this configurable via UI
    bench_ops::add_tool_to_bench(&focused, tool_name, "default")?;

    Ok(())
}

fn show_error(app: &gtk::Application, message: &str) {
    if let Some(window) = app.active_window() {
        let dialog = gtk::MessageDialog::builder()
            .transient_for(&window)
            .modal(true)
            .message_type(gtk::MessageType::Error)
            .buttons(gtk::ButtonsType::Close)
            .text(message)
            .build();
        dialog.connect_response(|dialog, _| dialog.close());
        dialog.present();
    }
}

mod gdk {
    pub use gtk4::gdk::Key;
}

mod pango {
    pub use gtk4::pango::EllipsizeMode;
}
