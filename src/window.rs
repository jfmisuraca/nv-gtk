use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gdk::{self, Key};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, EventControllerKey, Label, ListBox, ListBoxRow,
    Orientation, Paned, ScrolledWindow, SearchEntry, SelectionMode,
    TextTag, TextView,
};
use libadwaita::prelude::*;
use libadwaita::{Application, ApplicationWindow};

use crate::config::Config;
use crate::search::search_notes;
use crate::storage::StorageManager;
use crate::wiki_link::{extract_wiki_links, WikiLink};

pub struct AppState {
    pub config: Config,
    pub storage: StorageManager,
    pub filtered_indices: Vec<String>,
    pub current_note_id: Option<String>,
    pub save_timeout_source: Option<glib::SourceId>,
    pub is_updating_ui: bool,
    pub current_wiki_links: Vec<WikiLink>,
}

pub fn build_ui(app: &Application) {
    let config = Config::load();
    let storage = StorageManager::new(&config);
    let initial_filtered: Vec<String> = storage.notes.iter().map(|n| n.id.clone()).collect();

    let state = Rc::new(RefCell::new(AppState {
        config,
        storage,
        filtered_indices: initial_filtered,
        current_note_id: None,
        save_timeout_source: None,
        is_updating_ui: false,
        current_wiki_links: Vec::new(),
    }));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Notational Velocity")
        .default_width(900)
        .default_height(600)
        .build();

    // Split Pane (Paned)
    let paned = Paned::new(Orientation::Horizontal);
    paned.set_position(300);

    // Left Pane: Search Entry + Note List
    let left_box = GtkBox::new(Orientation::Vertical, 0);

    let search_entry = SearchEntry::builder()
        .placeholder_text("Buscar o crear nota (Enter)...")
        .margin_start(8)
        .margin_end(8)
        .margin_top(8)
        .margin_bottom(8)
        .build();

    left_box.append(&search_entry);

    let list_box = ListBox::new();
    list_box.set_selection_mode(SelectionMode::Single);
    list_box.add_css_class("navigation-sidebar");

    let list_scroll = ScrolledWindow::builder()
        .child(&list_box)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .build();

    left_box.append(&list_scroll);
    left_box.set_width_request(260);

    paned.set_start_child(Some(&left_box));

    // Right Pane: Editor & Status Bar
    let editor_box = GtkBox::new(Orientation::Vertical, 0);

    let text_view = TextView::builder()
        .wrap_mode(gtk4::WrapMode::WordChar)
        .monospace(false)
        .left_margin(16)
        .right_margin(16)
        .top_margin(16)
        .bottom_margin(16)
        .build();

    let text_scroll = ScrolledWindow::builder()
        .child(&text_view)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .build();

    editor_box.append(&text_scroll);

    // Status Footer
    let status_box = GtkBox::new(Orientation::Horizontal, 12);
    status_box.set_margin_start(16);
    status_box.set_margin_end(16);
    status_box.set_margin_top(6);
    status_box.set_margin_bottom(6);
    status_box.add_css_class("dim-label");

    let status_label = Label::new(Some("0 notas"));
    status_label.set_halign(Align::Start);

    let info_label = Label::new(Some(""));
    info_label.set_halign(Align::End);
    info_label.set_hexpand(true);

    status_box.append(&status_label);
    status_box.append(&info_label);
    editor_box.append(&status_box);

    paned.set_end_child(Some(&editor_box));
    window.set_content(Some(&paned));

    // Tag visual para los wiki-links (subrayado + color)
    let wiki_link_tag = TextTag::builder()
        .name("wiki-link")
        .underline(gtk4::pango::Underline::Single)
        .foreground("#4a9eff")
        .build();
    text_view.buffer().tag_table().add(&wiki_link_tag);

    // Recalcula los wiki-links del buffer actual, los guarda en el estado y los resalta visualmente
    let refresh_wiki_links = {
        let state = Rc::clone(&state);
        let text_view = text_view.clone();
        let wiki_link_tag = wiki_link_tag.clone();

        move || {
            let buffer = text_view.buffer();
            let (start, end) = buffer.bounds();
            let text = buffer.text(&start, &end, true).to_string();

            let links = extract_wiki_links(&text);

            // Limpiar tags viejos y volver a aplicar en las posiciones nuevas
            buffer.remove_tag(&wiki_link_tag, &start, &end);
            for link in &links {
                // start/end de WikiLink son offsets en bytes sobre &str;
                // TextBuffer::iter_at_offset espera offset en caracteres, así que convertimos.
                let char_start = text[..link.start].chars().count() as i32;
                let char_end = text[..link.end].chars().count() as i32;
                let iter_start = buffer.iter_at_offset(char_start);
                let iter_end = buffer.iter_at_offset(char_end);
                buffer.apply_tag(&wiki_link_tag, &iter_start, &iter_end);
            }

            state.borrow_mut().current_wiki_links = links;
        }
    };

    // Flushes any pending debounced save immediately (used when switching notes or closing)
    let flush_pending_save = {
        let state = Rc::clone(&state);
        let text_view = text_view.clone();

        move || {
            let mut st = state.borrow_mut();

            if let Some(source_id) = st.save_timeout_source.take() {
                source_id.remove();

                if let Some(current_id) = st.current_note_id.clone() {
                    let buffer = text_view.buffer();
                    let (start, end) = buffer.bounds();
                    let text = buffer.text(&start, &end, true).to_string();
                    st.storage.save_note(&current_id, &text);
                }
            }
        }
    };

    // Helper functions for UI refresh
    let populate_list = {
        let state = Rc::clone(&state);
        let list_box = list_box.clone();
        let status_label = status_label.clone();
        let search_entry = search_entry.clone();

        move || {
            // Remove all rows
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }

            let st = state.borrow();
            let query = search_entry.text().to_string();

            for id in &st.filtered_indices {
                if let Some(note) = st.storage.get_note(id) {
                    let row = ListBoxRow::new();
                    let row_box = GtkBox::new(Orientation::Vertical, 2);
                    row_box.set_margin_start(10);
                    row_box.set_margin_end(10);
                    row_box.set_margin_top(6);
                    row_box.set_margin_bottom(6);

                    let title_label = Label::new(Some(&note.title));
                    title_label.set_halign(Align::Start);
                    title_label.add_css_class("heading");

                    let meta_str = format!("{} • {}", note.formatted_date(), note.tags.join(" "));
                    let meta_label = Label::new(Some(&meta_str));
                    meta_label.set_halign(Align::Start);
                    meta_label.add_css_class("caption");
                    meta_label.add_css_class("dim-label");

                    row_box.append(&title_label);
                    row_box.append(&meta_label);
                    row.set_child(Some(&row_box));
                    list_box.append(&row);
                }
            }

            let count = st.filtered_indices.len();
            let total = st.storage.notes.len();
            if query.trim().is_empty() {
                status_label.set_text(&format!("{} notas en total", total));
            } else {
                status_label.set_text(&format!("{} de {} notas", count, total));
            }
        }
    };

    let select_note_by_id = {
        let state = Rc::clone(&state);
        let text_view = text_view.clone();
        let list_box = list_box.clone();
        let info_label = info_label.clone();
        let flush_pending_save = flush_pending_save.clone();
        let refresh_wiki_links = refresh_wiki_links.clone();

        move |target_id: &str| {
            // Guardar cualquier cambio pendiente de la nota anterior antes de cambiar
            flush_pending_save();

            let mut content_to_set = None;
            let mut pos_to_select = None;

            {
                let mut st = state.borrow_mut();
                st.is_updating_ui = true;
                if let Some(note) = st.storage.get_note(target_id) {
                    let id_clone = note.id.clone();
                    let content_clone = note.content.clone();
                    st.current_note_id = Some(id_clone);
                    content_to_set = Some(content_clone);
                    pos_to_select = st.filtered_indices.iter().position(|id| id == target_id);
                }
            }

            if let Some(content) = content_to_set {
                let buffer = text_view.buffer();
                buffer.set_text(&content);

                let words = content.split_whitespace().count();
                let chars = content.chars().count();
                info_label.set_text(&format!("{} palabras | {} caracteres", words, chars));

                if let Some(pos) = pos_to_select {
                    if let Some(row) = list_box.row_at_index(pos as i32) {
                        list_box.select_row(Some(&row));
                    }
                }
            }

            {
                let mut st = state.borrow_mut();
                st.is_updating_ui = false;
            }

            refresh_wiki_links();
        }
    };

    let update_search = {
        let state = Rc::clone(&state);
        let search_entry = search_entry.clone();
        let populate_list = populate_list.clone();
        let select_note_by_id = select_note_by_id.clone();

        move || {
            let query = search_entry.text().to_string();
            {
                let mut st = state.borrow_mut();
                st.filtered_indices = search_notes(&st.storage.notes, &query);
            }

            populate_list();

            // Auto select first match if any
            let first_id = {
                let st = state.borrow();
                st.filtered_indices.first().cloned()
            };

            if let Some(first_id) = first_id {
                select_note_by_id(&first_id);
            }
        }
    };

    // Navega hacia la nota referenciada por un wiki-link; la crea si no existe todavía
    let navigate_to_wiki_target = {
        let state = Rc::clone(&state);
        let populate_list = populate_list.clone();
        let select_note_by_id = select_note_by_id.clone();
        let search_entry = search_entry.clone();

        move |target: &str| {
            let target_clean = target.trim();
            if target_clean.is_empty() {
                return;
            }

            let action = {
                let mut st = state.borrow_mut();
                if let Some(existing) = st
                    .storage
                    .notes
                    .iter()
                    .find(|n| n.title.eq_ignore_ascii_case(target_clean))
                {
                    existing.id.clone()
                } else {
                    let new_note = st.storage.create_note(target_clean);
                    let new_id = new_note.id.clone();
                    st.filtered_indices = st.storage.notes.iter().map(|n| n.id.clone()).collect();
                    new_id
                }
            };

            search_entry.set_text("");
            populate_list();
            select_note_by_id(&action);
        }
    };

    // Initial population
    populate_list();
    let initial_first_id = {
        let st = state.borrow();
        st.storage.notes.first().map(|n| n.id.clone())
    };
    if let Some(first_id) = initial_first_id {
        select_note_by_id(&first_id);
    }

    // Connect Search Entry changed signal
    search_entry.connect_search_changed({
        let update_search = update_search.clone();
        move |_| {
            update_search();
        }
    });

    // Connect Search Entry Activate (Enter key in search bar)
    search_entry.connect_activate({
        let state = Rc::clone(&state);
        let search_entry = search_entry.clone();
        let text_view = text_view.clone();
        let populate_list = populate_list.clone();
        let select_note_by_id = select_note_by_id.clone();

        move |_| {
            let query = search_entry.text().to_string();
            let query_clean = query.trim();

            if query_clean.is_empty() {
                text_view.grab_focus();
                return;
            }

            let action = {
                let mut st = state.borrow_mut();
                if let Some(existing) = st
                    .storage
                    .notes
                    .iter()
                    .find(|n| n.title.eq_ignore_ascii_case(query_clean))
                {
                    Some((true, existing.id.clone()))
                } else if !st.filtered_indices.is_empty() {
                    Some((false, String::new()))
                } else {
                    let new_note = st.storage.create_note(query_clean);
                    let new_id = new_note.id.clone();
                    st.filtered_indices = st.storage.notes.iter().map(|n| n.id.clone()).collect();
                    Some((true, new_id))
                }
            };

            if let Some((should_select, id)) = action {
                if should_select {
                    populate_list();
                    select_note_by_id(&id);
                }
                text_view.grab_focus();
            }
        }
    });

    // Connect ListBox selection changed
    list_box.connect_row_selected({
        let state = Rc::clone(&state);
        let text_view = text_view.clone();
        let info_label = info_label.clone();
        let flush_pending_save = flush_pending_save.clone();
        let refresh_wiki_links = refresh_wiki_links.clone();

        move |_, row| {
            let note_data = if let Some(row) = row {
                let idx = row.index() as usize;
                if let Ok(st) = state.try_borrow() {
                    if st.is_updating_ui {
                        return;
                    }
                    st.filtered_indices.get(idx).and_then(|id| {
                        st.storage.get_note(id).map(|n| (n.id.clone(), n.content.clone()))
                    })
                } else {
                    None
                }
            } else {
                None
            };

            if let Some((id, content)) = note_data {
                // Guardar cualquier cambio pendiente de la nota anterior antes de cambiar
                flush_pending_save();

                if let Ok(mut st) = state.try_borrow_mut() {
                    st.current_note_id = Some(id);
                }
                let words = content.split_whitespace().count();
                let chars = content.chars().count();

                text_view.buffer().set_text(&content);
                info_label.set_text(&format!("{} palabras | {} caracteres", words, chars));

                refresh_wiki_links();
            }
        }
    });

    // Auto-Save when editing TextBuffer (debounced usando config.auto_save_ms)
    text_view.buffer().connect_changed({
        let state = Rc::clone(&state);
        let info_label = info_label.clone();
        let refresh_wiki_links = refresh_wiki_links.clone();

        move |buffer| {
            let mut st = state.borrow_mut();
            if st.is_updating_ui {
                return;
            }

            let text = {
                let (start, end) = buffer.bounds();
                buffer.text(&start, &end, true).to_string()
            };

            let words = text.split_whitespace().count();
            let chars = text.chars().count();
            info_label.set_text(&format!("{} palabras | {} caracteres", words, chars));

            // Cancelar el guardado pendiente anterior, si había
            if let Some(source_id) = st.save_timeout_source.take() {
                source_id.remove();
            }

            if let Some(current_id) = st.current_note_id.clone() {
                let delay_ms = st.config.auto_save_ms;
                let state_for_timeout = Rc::clone(&state);
                let buffer_for_timeout = buffer.clone();

                let source_id = glib::timeout_add_local(
                    std::time::Duration::from_millis(delay_ms),
                    move || {
                        let (start, end) = buffer_for_timeout.bounds();
                        let text = buffer_for_timeout.text(&start, &end, true).to_string();

                        let mut st = state_for_timeout.borrow_mut();
                        st.storage.save_note(&current_id, &text);
                        st.save_timeout_source = None;

                        glib::ControlFlow::Break // ejecutar una sola vez
                    },
                );

                st.save_timeout_source = Some(source_id);
            }

            drop(st);
            refresh_wiki_links();
        }
    });

    // New Note Action
    let create_new_empty_note = {
        let state = Rc::clone(&state);
        let search_entry = search_entry.clone();
        let populate_list = populate_list.clone();
        let select_note_by_id = select_note_by_id.clone();
        let text_view = text_view.clone();

        move || {
            search_entry.set_text("");
            let mut st = state.borrow_mut();
            let note = st.storage.create_note("Nueva Nota");
            let id = note.id.clone();
            st.filtered_indices = st.storage.notes.iter().map(|n| n.id.clone()).collect();
            drop(st);

            populate_list();
            select_note_by_id(&id);
            text_view.grab_focus();
        }
    };

    // Delete Note Button
    let delete_current_note = {
        let state = Rc::clone(&state);
        let search_entry = search_entry.clone();
        let update_search = update_search.clone();

        move || {
            let mut st = state.borrow_mut();
            if let Some(id) = st.current_note_id.clone() {
                st.storage.delete_note(&id);
                st.current_note_id = None;
                drop(st);

                search_entry.set_text("");
                update_search();
            }
        }
    };

    // Ctrl+Enter dentro del editor: si el cursor está sobre un [[wiki-link]], navega a esa nota
    let key_controller_editor = EventControllerKey::new();
    key_controller_editor.connect_key_pressed({
        let state = Rc::clone(&state);
        let text_view = text_view.clone();
        let navigate_to_wiki_target = navigate_to_wiki_target.clone();

        move |_, key, _, modifier| {
            let is_ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);

            if is_ctrl && (key == Key::Return || key == Key::KP_Enter) {
                let buffer = text_view.buffer();
                let cursor_offset = buffer.cursor_position();

                let target = {
                    let st = state.borrow();
                    st.current_wiki_links.iter().find_map(|link| {
                        let (s, e) = buffer.bounds();
                        let text = buffer.text(&s, &e, true).to_string();
                        let char_start = text[..link.start].chars().count() as i32;
                        let char_end = text[..link.end].chars().count() as i32;
                        if cursor_offset >= char_start && cursor_offset <= char_end {
                            Some(link.target.clone())
                        } else {
                            None
                        }
                    })
                };

                if let Some(target) = target {
                    navigate_to_wiki_target(&target);
                    return glib::Propagation::Stop;
                }
            }

            glib::Propagation::Proceed
        }
    });
    text_view.add_controller(key_controller_editor);

    // Keyboard Controller for Global App Shortcuts (Ctrl+L, Esc, Ctrl+N, Ctrl+D)
    let key_controller = EventControllerKey::new();
    key_controller.connect_key_pressed({
        let search_entry = search_entry.clone();
        let _list_box = list_box.clone();
        let create_new_empty_note = create_new_empty_note.clone();
        let delete_current_note = delete_current_note.clone();

        move |_, key, _, modifier| {
            let is_ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);

            match key {
                Key::l if is_ctrl => {
                    search_entry.grab_focus();
                    search_entry.select_region(0, -1);
                    glib::Propagation::Stop
                }
                Key::f if is_ctrl => {
                    search_entry.grab_focus();
                    search_entry.select_region(0, -1);
                    glib::Propagation::Stop
                }
                Key::n if is_ctrl => {
                    create_new_empty_note();
                    glib::Propagation::Stop
                }
                Key::d if is_ctrl => {
                    delete_current_note();
                    glib::Propagation::Stop
                }
                Key::Escape => {
                    search_entry.grab_focus();
                    search_entry.select_region(0, -1);
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        }
    });

    window.add_controller(key_controller);

    // Guardar cualquier cambio pendiente al cerrar la ventana
    window.connect_close_request({
        let flush_pending_save = flush_pending_save.clone();
        move |_| {
            flush_pending_save();
            glib::Propagation::Proceed
        }
    });

    window.present();
}
