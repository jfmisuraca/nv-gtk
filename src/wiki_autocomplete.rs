use std::cell::RefCell;
use std::rc::Rc;

use gtk4::gdk::{self, Key};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, CssProvider, EventControllerKey, Label, ListBox, ListBoxRow,
    Orientation, Overlay, ScrolledWindow, SelectionMode, Separator, TextTag, TextView,
    TextWindowType,
};

use crate::app_state::{timestamp_title, AppState};
use crate::wiki_link::extract_wiki_links;

/// Estado del panel de autocompletado de wiki-links
#[derive(Clone, Default)]
struct AutocompleteState {
    active: bool,
    start_offset: i32, // offset (en caracteres) justo después de "[["
    selected: usize,
    query: String, // lo que se escribió después de "[[", para poder crear una nota con eso
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

/// Encapsula todo el sistema de wiki-links: resaltado visual de `[[links]]` cerrados
/// y un panel flotante de autocompletado difuso mientras se escribe `[[query`.
///
/// Se conecta a un `TextView` existente y superpone su panel sobre el `Overlay`
/// que envuelve al editor. Expone `refresh()`, `update()` y `close()` para que
/// el resto de la app (autoguardado, cambio de nota) pueda coordinarse con él.
#[derive(Clone)]
pub struct WikiAutocomplete {
    refresh_fn: Rc<dyn Fn()>,
    update_fn: Rc<dyn Fn()>,
    close_fn: Rc<dyn Fn()>,
}

impl WikiAutocomplete {
    /// Recalcula los wiki-links del buffer actual y los resalta visualmente.
    /// Llamar después de cualquier cambio en el contenido de la nota.
    pub fn refresh(&self) {
        (self.refresh_fn)();
    }

    /// Revisa si el cursor quedó dentro de un `[[query` sin cerrar y, si es así,
    /// abre/actualiza el panel de autocompletado. Llamar en cada cambio del buffer.
    pub fn update(&self) {
        (self.update_fn)();
    }

    /// Cierra el panel de autocompletado si estaba abierto, sin insertar nada.
    pub fn close(&self) {
        (self.close_fn)();
    }

    /// Conecta el sistema de wiki-links a `text_view`, superponiendo su panel de
    /// autocompletado sobre `editor_overlay`.
    ///
    /// `on_navigate` se invoca cuando el usuario hace `Ctrl+Enter` con el cursor
    /// sobre un `[[link]]` ya cerrado; el caller decide qué hacer con el texto del
    /// link (por ejemplo, volcarlo en la barra de búsqueda).
    pub fn setup(
        state: Rc<RefCell<AppState>>,
        text_view: TextView,
        editor_overlay: Overlay,
        on_navigate: impl Fn(&str) + 'static,
    ) -> Self {
        let on_navigate = Rc::new(on_navigate);

        // Tag visual para los wiki-links (subrayado + color)
        let wiki_link_tag = TextTag::builder()
            .name("wiki-link")
            .underline(gtk4::pango::Underline::Single)
            .foreground("#4a9eff")
            .build();
        text_view.buffer().tag_table().add(&wiki_link_tag);

        // --- Panel de autocompletado de wiki-links ---
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
        autocomplete_body.add_css_class("nv-autocomplete-panel");
        autocomplete_body.set_halign(Align::Start);
        autocomplete_body.set_valign(Align::Start);
        autocomplete_body.set_visible(false);

        // CSS propio para que el panel tenga fondo y borde (al no ser un Popover nativo,
        // no hereda el estilo ".popover" del tema automáticamente)
        let css_provider = CssProvider::new();
        css_provider.load_from_string(
            ".nv-autocomplete-panel { \
                background-color: @theme_base_color; \
                border: 1px solid alpha(@borders, 0.8); \
                border-radius: 8px; \
            }",
        );
        if let Some(display) = gtk4::gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &css_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }

        editor_overlay.add_overlay(&autocomplete_body);

        // Resalta visualmente la fila seleccionada dentro del panel
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

        // Cierra el panel de autocompletado sin insertar nada
        let close_autocomplete: Rc<dyn Fn()> = {
            let autocomplete_body = autocomplete_body.clone();
            let autocomplete_state = Rc::clone(&autocomplete_state);
            Rc::new(move || {
                autocomplete_body.set_visible(false);
                autocomplete_state.borrow_mut().active = false;
            })
        };

        // Recalcula los wiki-links del buffer actual, los guarda en el estado y los resalta visualmente
        let refresh_wiki_links: Rc<dyn Fn()> = {
            let state = Rc::clone(&state);
            let text_view = text_view.clone();
            let wiki_link_tag = wiki_link_tag.clone();

            Rc::new(move || {
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
            })
        };

        // Detecta si el cursor está dentro de un "[[query" sin cerrar, y si es así
        // abre/actualiza el panel con las notas candidatas (fuzzy match).
        let update_autocomplete: Rc<dyn Fn()> = {
            let state = Rc::clone(&state);
            let text_view = text_view.clone();
            let autocomplete_state = Rc::clone(&autocomplete_state);
            let autocomplete_matches = Rc::clone(&autocomplete_matches);
            let autocomplete_body = autocomplete_body.clone();
            let editor_overlay = editor_overlay.clone();
            let autocomplete_list = autocomplete_list.clone();
            let select_autocomplete_row = select_autocomplete_row.clone();
            let close_autocomplete = Rc::clone(&close_autocomplete);
            let update_autocomplete_preview = update_autocomplete_preview.clone();

            Rc::new(move || {
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

                // Calcular matches difusos contra la primera línea del contenido de cada nota
                // (el título es solo un timestamp, no sirve para buscar ni mostrar)
                let mut matches: Vec<(String, String, String, String, i32)> = {
                    let st = state.borrow();
                    st.storage
                        .notes
                        .iter()
                        .filter_map(|note| {
                            let display_title = note
                                .content
                                .lines()
                                .map(|l| l.trim())
                                .find(|l| !l.is_empty())
                                .unwrap_or("(nota vacía)")
                                .to_string();

                            fuzzy_score(&query, &display_title).map(|score| {
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
                                (note.id.clone(), note.title.clone(), display_title, tags, score)
                            })
                        })
                        .collect()
                };

                if matches.is_empty() {
                    while let Some(child) = autocomplete_list.first_child() {
                        autocomplete_list.remove(&child);
                    }
                    let empty_row = ListBoxRow::new();
                    empty_row.set_selectable(false);
                    empty_row.set_activatable(false);
                    let empty_label = Label::new(Some("Sin coincidencias"));
                    empty_label.add_css_class("dim-label");
                    empty_label.set_margin_start(10);
                    empty_label.set_margin_end(10);
                    empty_label.set_margin_top(6);
                    empty_label.set_margin_bottom(6);
                    empty_row.set_child(Some(&empty_label));
                    autocomplete_list.append(&empty_row);

                    *autocomplete_matches.borrow_mut() = Vec::new();
                    *autocomplete_state.borrow_mut() = AutocompleteState {
                        active: true,
                        start_offset: start_char_offset,
                        selected: 0,
                        query: query.clone(),
                    };

                    let anchor_iter = buffer.iter_at_offset(start_char_offset);
                    let rect = text_view.iter_location(&anchor_iter);
                    let (wx, wy) = text_view
                        .buffer_to_window_coords(TextWindowType::Widget, rect.x(), rect.y());
                    let src_point = gtk4::graphene::Point::new(wx as f32, (wy + rect.height()) as f32);
                    let (ox, oy) = text_view
                        .compute_point(&editor_overlay, &src_point)
                        .map(|p| (p.x() as f64, p.y() as f64))
                        .unwrap_or((wx as f64, (wy + rect.height()) as f64));
                    autocomplete_body.set_margin_start(ox as i32);
                    autocomplete_body.set_margin_top(oy as i32);
                    autocomplete_body.set_visible(true);
                    return;
                }

                matches.sort_by(|a, b| a.4.cmp(&b.4).then_with(|| a.2.cmp(&b.2)));
                matches.truncate(8);

                // Poblar la lista visual
                while let Some(child) = autocomplete_list.first_child() {
                    autocomplete_list.remove(&child);
                }

                for (_id, _title, display_title, tags, _score) in &matches {
                    let row = ListBoxRow::new();
                    let row_box = GtkBox::new(Orientation::Horizontal, 8);
                    row_box.set_margin_start(10);
                    row_box.set_margin_end(10);
                    row_box.set_margin_top(4);
                    row_box.set_margin_bottom(4);

                    let title_label = Label::new(Some(&format!("# {}", display_title)));
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
                    .map(|(id, title, _display, tags, _)| (id, title, tags))
                    .collect();

                select_autocomplete_row(0);
                update_autocomplete_preview(0);
                *autocomplete_state.borrow_mut() = AutocompleteState {
                    active: true,
                    start_offset: start_char_offset,
                    selected: 0,
                    query: query.clone(),
                };

                // Posicionar el panel justo debajo de donde se escribió "[["
                let anchor_iter = buffer.iter_at_offset(start_char_offset);
                let rect = text_view.iter_location(&anchor_iter);
                let (wx, wy) =
                    text_view.buffer_to_window_coords(TextWindowType::Widget, rect.x(), rect.y());
                let src_point = gtk4::graphene::Point::new(wx as f32, (wy + rect.height()) as f32);
                let (ox, oy) = text_view
                    .compute_point(&editor_overlay, &src_point)
                    .map(|p| (p.x() as f64, p.y() as f64))
                    .unwrap_or((wx as f64, (wy + rect.height()) as f64));
                autocomplete_body.set_margin_start(ox as i32);
                autocomplete_body.set_margin_top(oy as i32);
                autocomplete_body.set_visible(true);
            })
        };

        // Inserta el título seleccionado en el buffer, reemplazando el "[[query" pendiente
        let insert_autocomplete_selection = {
            let text_view = text_view.clone();
            let autocomplete_state = Rc::clone(&autocomplete_state);
            let autocomplete_matches = Rc::clone(&autocomplete_matches);
            let close_autocomplete = Rc::clone(&close_autocomplete);

            move |idx: usize| {
                let title = {
                    let matches = autocomplete_matches.borrow();
                    matches.get(idx).map(|(_, t, _)| t.clone())
                };

                let title = match title {
                    Some(t) => t,
                    None => return,
                };

                let ac_state = autocomplete_state.borrow().clone();
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

        // Crea una nota nueva usando la query actual como primera línea de contenido,
        // e inserta el link a esa nota nueva en el buffer (Ctrl+Shift+Enter)
        let create_note_from_query = {
            let state = Rc::clone(&state);
            let text_view = text_view.clone();
            let autocomplete_state = Rc::clone(&autocomplete_state);
            let close_autocomplete = Rc::clone(&close_autocomplete);

            move || {
                let ac_state = autocomplete_state.borrow().clone();
                let query_clean = ac_state.query.trim();
                if query_clean.is_empty() {
                    return;
                }

                let new_title = {
                    let mut st = state.borrow_mut();
                    let new_note = st.storage.create_note(&timestamp_title());
                    let new_id = new_note.id.clone();
                    st.storage.save_note(&new_id, query_clean);
                    new_id
                };

                let buffer = text_view.buffer();
                let cursor_offset = buffer.cursor_position();

                let mut start_iter = buffer.iter_at_offset(ac_state.start_offset);
                let mut end_iter = buffer.iter_at_offset(cursor_offset);
                buffer.delete(&mut start_iter, &mut end_iter);
                buffer.insert(&mut start_iter, &format!("{}]]", new_title));
                buffer.place_cursor(&start_iter);

                close_autocomplete();
                text_view.grab_focus();
            }
        };

        // Controlador de teclado del editor: maneja el panel de autocompletado
        // (↑/↓/Enter/Tab/Esc) y, si no está activo, Ctrl+Enter para navegar wiki-links existentes.
        let key_controller_editor = EventControllerKey::new();
        key_controller_editor.connect_key_pressed({
            let state = Rc::clone(&state);
            let text_view = text_view.clone();
            let on_navigate = Rc::clone(&on_navigate);
            let autocomplete_state = Rc::clone(&autocomplete_state);
            let autocomplete_matches = Rc::clone(&autocomplete_matches);
            let select_autocomplete_row = select_autocomplete_row.clone();
            let update_autocomplete_preview = update_autocomplete_preview.clone();
            let insert_autocomplete_selection = insert_autocomplete_selection.clone();
            let create_note_from_query = create_note_from_query.clone();
            let close_autocomplete = Rc::clone(&close_autocomplete);

            move |_, key, _, modifier| {
                let is_ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);
                let is_shift = modifier.contains(gdk::ModifierType::SHIFT_MASK);

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
                        Key::Return | Key::KP_Enter if is_ctrl && is_shift => {
                            create_note_from_query();
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
                        on_navigate(&target);
                        return glib::Propagation::Stop;
                    }
                }

                glib::Propagation::Proceed
            }
        });
        text_view.add_controller(key_controller_editor);

        Self {
            refresh_fn: refresh_wiki_links,
            update_fn: update_autocomplete,
            close_fn: close_autocomplete,
        }
    }
}
