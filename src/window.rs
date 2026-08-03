use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gdk::{self, Key};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, EventControllerKey, Label, ListBox, ListBoxRow,
    Orientation, Paned, Popover, PositionType, ScrolledWindow, SearchEntry,
    SelectionMode, Separator, TextTag, TextView, TextWindowType,
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

// Estado del popover de autocompletado de wiki-links
#[derive(Clone, Copy)]
struct AutocompleteState {
    active: bool,
    start_offset: i32, // offset (en caracteres) justo después de "[["
    selected: usize,
}

impl Default for AutocompleteState {
    fn default() -> Self {
        Self {
            active: false,
            start_offset: 0,
            selected: 0,
        }
    }
}

/// Coincidencia difusa tipo "subsequence": todos los caracteres de `query` deben
/// aparecer en `target` en el mismo orden, aunque no estén seguidos.
/// Devuelve `None` si no matchea, o `Some(score)` si matchea (menor score = mejor).
fn fuzzy_score(query: &str, target: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }

    let query_lower = query.to_lowercase();
    let target_lower = target.to_lowercase();

    let mut score: i32 = 0;
    let mut last_match: Option<usize> = None;
    let mut q_chars = query_lower.chars().peekable();

    for (ti, tc) in target_lower.chars().enumerate() {
        if let Some(&qc) = q_chars.peek() {
            if tc == qc {
                match last_match {
                    Some(last) => score += (ti - last - 1) as i32, // penaliza huecos entre matches
                    None => score += ti as i32, // penaliza empezar lejos del principio
                }
                last_match = Some(ti);
                q_chars.next();
            }
        } else {
            break;
        }
    }

    if q_chars.peek().is_some() {
        None // no todos los caracteres de la query aparecieron
    } else {
        Some(score)
    }
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
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
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

    // --- Popover de autocompletado de wiki-links ---
    let autocomplete_state: Rc<RefCell<AutocompleteState>> =
        Rc::new(RefCell::new(AutocompleteState::default()));
    let autocomplete_matches: Rc<RefCell<Vec<(String, String, String)>>> =
        Rc::new(RefCell::new(Vec::new())); // (id, título, tags formateados)

    let autocomplete_list = ListBox::new();
    autocomplete_list.set_selection_mode(SelectionMode::Single);
    autocomplete_list.add_css_class("navigation-sidebar");

    let autocomplete_scroll = ScrolledWindow::builder()
        .child(&autocomplete_list)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .max_content_height(240)
        .propagate_natural_height(true)
        .width_request(260)
        .build();

    // Preview de las primeras líneas de la nota seleccionada
    let autocomplete_preview = Label::new(None);
    autocomplete_preview.set_halign(Align::Start);
    autocomplete_preview.set_valign(Align::Start);
    autocomplete_preview.set_wrap(true);
    autocomplete_preview.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
    autocomplete_preview.set_max_width_chars(80);
    autocomplete_preview.set_width_chars(80);
    autocomplete_preview.set_xalign(0.0);
    autocomplete_preview.set_margin_start(10);
    autocomplete_preview.set_margin_end(10);
    autocomplete_preview.set_margin_top(8);
    autocomplete_preview.set_margin_bottom(8);
    autocomplete_preview.add_css_class("caption");
    autocomplete_preview.add_css_class("dim-label");

    let autocomplete_body = GtkBox::new(Orientation::Horizontal, 0);
    autocomplete_body.append(&autocomplete_scroll);
    autocomplete_body.append(&Separator::new(Orientation::Vertical));
    autocomplete_body.append(&autocomplete_preview);

    let autocomplete_popover = Popover::new();
    autocomplete_popover.set_parent(&text_view);
    autocomplete_popover.set_child(Some(&autocomplete_body));
    autocomplete_popover.set_position(PositionType::Bottom);
    autocomplete_popover.set_autohide(false);

    // Resalta visualmente la fila seleccionada dentro del popover
    let select_autocomplete_row = {
        let autocomplete_list = autocomplete_list.clone();
        move |idx: usize| {
            if let Some(row) = autocomplete_list.row_at_index(idx as i32) {
                autocomplete_list.select_row(Some(&row));
            }
        }
    };

    // Muestra en el panel lateral las primeras 4 líneas del contenido de la nota candidata
    let update_autocomplete_preview = {
        let state = Rc::clone(&state);
        let autocomplete_matches = Rc::clone(&autocomplete_matches);
        let autocomplete_preview = autocomplete_preview.clone();

        move |idx: usize| {
            let note_id = {
                let matches = autocomplete_matches.borrow();
                matches.get(idx).map(|(id, _, _)| id.clone())
            };

            let preview_text = note_id.and_then(|id| {
                let st = state.borrow();
                st.storage.get_note(&id).map(|note| {
                    note.content
                        .lines()
                        .take(4)
                        .collect::<Vec<_>>()
                        .join("\n")
                })
            });

            autocomplete_preview.set_text(preview_text.as_deref().unwrap_or(""));
        }
    };

    // Cierra el popover de autocompletado sin insertar nada
    let close_autocomplete = {
        let autocomplete_popover = autocomplete_popover.clone();
        let autocomplete_state = Rc::clone(&autocomplete_state);
        move || {
            autocomplete_popover.popdown();
            autocomplete_state.borrow_mut().active = false;
        }
    };

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

    // Detecta si el cursor está dentro de un "[[query" sin cerrar, y si es así
    // abre/actualiza el popover con las notas candidatas (fuzzy match).
    let update_autocomplete = {
        let state = Rc::clone(&state);
        let text_view = text_view.clone();
        let autocomplete_state = Rc::clone(&autocomplete_state);
        let autocomplete_matches = Rc::clone(&autocomplete_matches);
        let autocomplete_popover = autocomplete_popover.clone();
        let autocomplete_list = autocomplete_list.clone();
        let select_autocomplete_row = select_autocomplete_row.clone();
        let close_autocomplete = close_autocomplete.clone();
        let update_autocomplete_preview = update_autocomplete_preview.clone();

        move || {
            let buffer = text_view.buffer();
            let cursor_offset = buffer.cursor_position();
            let (buf_start, _) = buffer.bounds();
            let cursor_iter = buffer.iter_at_offset(cursor_offset);
            let text_before_cursor = buffer.text(&buf_start, &cursor_iter, true).to_string();

            // Solo miramos la línea actual
            let line_start_byte = text_before_cursor.rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_before_cursor = &text_before_cursor[line_start_byte..];

            let context = line_before_cursor.rfind("[[").and_then(|idx_byte| {
                let after = &line_before_cursor[idx_byte + 2..];
                if after.contains("]]") {
                    None
                } else {
                    Some((idx_byte, after.to_string()))
                }
            });

            let (bracket_idx_byte, query) = match context {
                Some(c) => c,
                None => {
                    close_autocomplete();
                    return;
                }
            };

            // Convertir el offset en bytes (dentro de text_before_cursor) a offset en caracteres
            let absolute_byte_offset = line_start_byte + bracket_idx_byte + 2;
            let start_char_offset =
                text_before_cursor[..absolute_byte_offset].chars().count() as i32;

            // Calcular matches difusos contra los títulos de las notas
            let mut matches: Vec<(String, String, String, i32)> = {
                let st = state.borrow();
                st.storage
                    .notes
                    .iter()
                    .filter_map(|note| {
                        fuzzy_score(&query, &note.title).map(|score| {
                            let tags = if note.tags.is_empty() {
                                String::new()
                            } else {
                                format!(
                                    "[{}]",
                                    note.tags
                                        .iter()
                                        .map(|t| t.as_str())
                                        .collect::<Vec<_>>()
                                        .join(" ")
                                )
                            };
                            (note.id.clone(), note.title.clone(), tags, score)
                        })
                    })
                    .collect()
            };

            if matches.is_empty() {
                close_autocomplete();
                return;
            }

            matches.sort_by(|a, b| a.3.cmp(&b.3).then_with(|| a.1.cmp(&b.1)));
            matches.truncate(8);

            // Poblar la lista visual
            while let Some(child) = autocomplete_list.first_child() {
                autocomplete_list.remove(&child);
            }

            for (_id, title, tags, _score) in &matches {
                let row = ListBoxRow::new();
                let row_box = GtkBox::new(Orientation::Horizontal, 8);
                row_box.set_margin_start(10);
                row_box.set_margin_end(10);
                row_box.set_margin_top(4);
                row_box.set_margin_bottom(4);

                let title_label = Label::new(Some(&format!("# {}", title)));
                title_label.set_halign(Align::Start);
                title_label.set_hexpand(true);

                let tags_label = Label::new(Some(tags));
                tags_label.set_halign(Align::End);
                tags_label.add_css_class("dim-label");
                tags_label.add_css_class("caption");

                row_box.append(&title_label);
                row_box.append(&tags_label);
                row.set_child(Some(&row_box));
                autocomplete_list.append(&row);
            }

            *autocomplete_matches.borrow_mut() = matches
                .into_iter()
                .map(|(id, title, tags, _)| (id, title, tags))
                .collect();

            select_autocomplete_row(0);
            update_autocomplete_preview(0);
            *autocomplete_state.borrow_mut() = AutocompleteState {
                active: true,
                start_offset: start_char_offset,
                selected: 0,
            };

            // Posicionar el popover justo debajo de donde se escribió "[["
            let anchor_iter = buffer.iter_at_offset(start_char_offset);
            let rect = text_view.iter_location(&anchor_iter);
            let (wx, wy) =
                text_view.buffer_to_window_coords(TextWindowType::Widget, rect.x(), rect.y());
            let pointing_rect = gdk::Rectangle::new(wx, wy + rect.height(), 1, 1);
            autocomplete_popover.set_pointing_to(Some(&pointing_rect));
            autocomplete_popover.popup();
        }
    };

    // Inserta el título seleccionado en el buffer, reemplazando el "[[query" pendiente
    let insert_autocomplete_selection = {
        let text_view = text_view.clone();
        let autocomplete_state = Rc::clone(&autocomplete_state);
        let autocomplete_matches = Rc::clone(&autocomplete_matches);
        let close_autocomplete = close_autocomplete.clone();

        move |idx: usize| {
            let title = {
                let matches = autocomplete_matches.borrow();
                matches.get(idx).map(|(_, t, _)| t.clone())
            };

            let title = match title {
                Some(t) => t,
                None => return,
            };

            let ac_state = *autocomplete_state.borrow();
            let buffer = text_view.buffer();
            let cursor_offset = buffer.cursor_position();

            let mut start_iter = buffer.iter_at_offset(ac_state.start_offset);
            let mut end_iter = buffer.iter_at_offset(cursor_offset);
            buffer.delete(&mut start_iter, &mut end_iter);
            buffer.insert(&mut start_iter, &format!("{}]]", title));
            buffer.place_cursor(&start_iter);

            close_autocomplete();
            text_view.grab_focus();
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
        let close_autocomplete = close_autocomplete.clone();

        move |target_id: &str| {
            close_autocomplete();
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

        move || {
            let query = search_entry.text().to_string();
            {
                let mut st = state.borrow_mut();
                st.filtered_indices = search_notes(&st.storage.notes, &query);
            }

            populate_list();
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
                if let Some(best_match_id) = st.filtered_indices.first().cloned() {
                    (true, best_match_id)
                } else {
                    let new_note = st.storage.create_note(query_clean);
                    let new_id = new_note.id.clone();
                    st.filtered_indices = st.storage.notes.iter().map(|n| n.id.clone()).collect();
                    (true, new_id)
                }
            };

            let (should_select, id) = action;
            if should_select {
                populate_list();
                select_note_by_id(&id);
            }
            text_view.grab_focus();
        }
    });

    // Al activar una fila (Enter o doble clic) se carga la nota en el editor.
    // Navegar con las flechas solo mueve la selección visual, sin abrir la nota.
    list_box.connect_row_activated({
        let state = Rc::clone(&state);
        let select_note_by_id = select_note_by_id.clone();

        move |_, row| {
            let idx = row.index() as usize;
            let target_id = {
                let st = state.borrow();
                st.filtered_indices.get(idx).cloned()
            };

            if let Some(id) = target_id {
                select_note_by_id(&id);
            }
        }
    });

    // Auto-Save when editing TextBuffer (debounced usando config.auto_save_ms)
    text_view.buffer().connect_changed({
        let state = Rc::clone(&state);
        let info_label = info_label.clone();
        let refresh_wiki_links = refresh_wiki_links.clone();
        let update_autocomplete = update_autocomplete.clone();

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
            update_autocomplete();
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

    // Mueve la selección de la lista de notas hacia abajo (+1) o arriba (-1) y la abre
    let move_list_selection = {
        let state = Rc::clone(&state);
        let list_box = list_box.clone();
        let select_note_by_id = select_note_by_id.clone();

        move |delta: i32| {
            let current_index = list_box
                .selected_row()
                .map(|row| row.index())
                .unwrap_or(-1);

            let target_index = current_index + delta;
            if target_index < 0 {
                return;
            }

            if let Some(row) = list_box.row_at_index(target_index) {
                list_box.select_row(Some(&row));
                row.grab_focus();

                let idx = target_index as usize;
                let target_id = {
                    let st = state.borrow();
                    st.filtered_indices.get(idx).cloned()
                };
                if let Some(id) = target_id {
                    select_note_by_id(&id);
                }
            }
        }
    };

    // Controlador de teclado del editor: maneja el popover de autocompletado
    // (↑/↓/Enter/Tab/Esc) y, si no está activo, Ctrl+Enter para navegar wiki-links existentes.
    let key_controller_editor = EventControllerKey::new();
    key_controller_editor.connect_key_pressed({
        let state = Rc::clone(&state);
        let text_view = text_view.clone();
        let navigate_to_wiki_target = navigate_to_wiki_target.clone();
        let autocomplete_state = Rc::clone(&autocomplete_state);
        let autocomplete_matches = Rc::clone(&autocomplete_matches);
        let select_autocomplete_row = select_autocomplete_row.clone();
        let update_autocomplete_preview = update_autocomplete_preview.clone();
        let insert_autocomplete_selection = insert_autocomplete_selection.clone();
        let close_autocomplete = close_autocomplete.clone();

        move |_, key, _, modifier| {
            let is_ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);

            let ac_active = autocomplete_state.borrow().active;
            if ac_active {
                let count = autocomplete_matches.borrow().len();
                match key {
                    Key::Down => {
                        let mut ac = autocomplete_state.borrow_mut();
                        if count > 0 {
                            ac.selected = (ac.selected + 1).min(count - 1);
                        }
                        let sel = ac.selected;
                        drop(ac);
                        select_autocomplete_row(sel);
                        update_autocomplete_preview(sel);
                        return glib::Propagation::Stop;
                    }
                    Key::Up => {
                        let mut ac = autocomplete_state.borrow_mut();
                        ac.selected = ac.selected.saturating_sub(1);
                        let sel = ac.selected;
                        drop(ac);
                        select_autocomplete_row(sel);
                        update_autocomplete_preview(sel);
                        return glib::Propagation::Stop;
                    }
                    Key::Return | Key::KP_Enter | Key::Tab => {
                        let sel = autocomplete_state.borrow().selected;
                        insert_autocomplete_selection(sel);
                        return glib::Propagation::Stop;
                    }
                    Key::Escape => {
                        close_autocomplete();
                        return glib::Propagation::Stop;
                    }
                    _ => {
                        // Dejar pasar la tecla para que se siga escribiendo la query;
                        // connect_changed va a recalcular los matches.
                        return glib::Propagation::Proceed;
                    }
                }
            }

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
        let move_list_selection = move_list_selection.clone();

        move |_, key, _, modifier| {
            let is_ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);

            match key {
                Key::j if is_ctrl => {
                    move_list_selection(1);
                    glib::Propagation::Stop
                }
                Key::k if is_ctrl => {
                    move_list_selection(-1);
                    glib::Propagation::Stop
                }
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
