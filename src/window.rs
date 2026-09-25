use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk4::gdk::{self, Key};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, CheckButton, Entry, EventControllerKey, Label, ListBox,
    ListBoxRow, MenuButton, Orientation, Overlay, Paned, Popover, ScrolledWindow, SearchEntry,
    SelectionMode, TextView, Window,
};
use libadwaita::prelude::*;
use libadwaita::{Application, ApplicationWindow};

use crate::app_state::AppState;
use crate::palettes::ThemeId;
use crate::theme;
use crate::wiki_autocomplete::WikiAutocomplete;
use nv_core::config::Config;
use nv_core::search::search_notes;
use nv_core::storage::StorageManager;
use nv_core::util::timestamp_title;

/// Handles a los widgets clave de la UI. Los expone `build_ui` para poder
/// probar el comportamiento responsivo sin depender del display server.
#[derive(Clone)]
#[allow(dead_code)]
pub struct UiHandles {
    pub window: ApplicationWindow,
    pub paned: Paned,
    pub list_scroll: ScrolledWindow,
    pub search_entry: SearchEntry,
    pub list_box: ListBox,
    pub results_panel: GtkBox,
    pub results_list_box: ListBox,
    pub text_view: TextView,
    pub is_portrait: Rc<Cell<bool>>,
    pub apply_layout: Rc<dyn Fn(bool)>,
    pub update_results_visibility: Rc<dyn Fn()>,
}

/// Small modal confirm dialog: title + body + Cancel/confirm-button.
/// Runs `on_confirm` only when the user picks the confirm action. Used for
/// permanent deletions (purge, empty trash); moving to trash needs none.
fn confirm_dialog(
    parent: &Window,
    title: &str,
    body: &str,
    confirm_label: &str,
    on_confirm: impl Fn() + 'static,
) {
    let dialog = Window::builder()
        .transient_for(parent)
        .modal(true)
        .title(title)
        .default_width(340)
        .build();
    let vbox = GtkBox::new(Orientation::Vertical, 8);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    dialog.set_child(Some(&vbox));
    let label = Label::new(Some(body));
    label.set_wrap(true);
    vbox.append(&label);
    let buttons = GtkBox::new(Orientation::Horizontal, 6);
    buttons.set_halign(Align::End);
    let cancel_btn = Button::with_label("Cancelar");
    let ok_btn = Button::with_label(confirm_label);
    buttons.append(&cancel_btn);
    buttons.append(&ok_btn);
    vbox.append(&buttons);

    let dialog_c = dialog.clone();
    cancel_btn.connect_clicked(move |_| dialog_c.close());
    let dialog_c = dialog.clone();
    ok_btn.connect_clicked(move |_| {
        dialog_c.close();
        on_confirm();
    });
    // Keyboard: Enter confirms via the default widget, Esc cancels.
    dialog.set_default_widget(Some(&ok_btn));
    let esc_controller = EventControllerKey::new();
    let dialog_c = dialog.clone();
    esc_controller.connect_key_pressed(move |_, key, _, _| {
        if key == Key::Escape {
            dialog_c.close();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    dialog.add_controller(esc_controller);
    dialog.present();
}

/// Rebuilds the trash dialog rows from `trash_notes`, wiring per-row
/// restore/purge buttons that sync the main list (`update_search`) and then
/// rebuild themselves. Free function (not a closure) so row handlers can
/// re-enter it without reference cycles.
fn rebuild_trash_rows(
    list: &ListBox,
    state: &Rc<RefCell<AppState>>,
    update_search: &Rc<dyn Fn()>,
    parent: &Window,
) {
    while let Some(row) = list.first_child() {
        list.remove(&row);
    }
    let trashed: Vec<(String, String)> = {
        let st = state.borrow();
        st.storage
            .trash_notes()
            .iter()
            .map(|n| (n.id.clone(), n.title.clone()))
            .collect()
    };
    for (id, title) in trashed {
        let row = ListBoxRow::new();
        let hbox = GtkBox::new(Orientation::Horizontal, 6);
        let label = Label::new(Some(&title));
        label.set_halign(Align::Start);
        label.set_hexpand(true);
        let restore_btn = Button::with_label("Restaurar");
        let purge_btn = Button::with_label("Eliminar");
        hbox.append(&label);
        hbox.append(&restore_btn);
        hbox.append(&purge_btn);
        row.set_child(Some(&hbox));
        list.append(&row);

        let list_c = list.clone();
        let state_c = Rc::clone(state);
        let update_search_c = Rc::clone(update_search);
        let parent_c = parent.clone();
        let id_c = id.clone();
        restore_btn.connect_clicked(move |_| {
            {
                let mut st = state_c.borrow_mut();
                let _ = st.storage.restore_note(&id_c);
            }
            update_search_c();
            rebuild_trash_rows(&list_c, &state_c, &update_search_c, &parent_c);
        });

        let list_c = list.clone();
        let state_c = Rc::clone(state);
        let update_search_c = Rc::clone(update_search);
        let parent_c = parent.clone();
        let id_c = id.clone();
        let title_c = title.clone();
        purge_btn.connect_clicked(move |_| {
            let list_c = list_c.clone();
            let state_c = Rc::clone(&state_c);
            let update_search_c = update_search_c.clone();
            let dialog_parent = parent_c.clone();
            let confirm_parent = parent_c.clone();
            let id_c2 = id_c.clone();
            confirm_dialog(
                &dialog_parent,
                "Eliminar definitivamente",
                &format!("¿Borrar \"{title_c}\" para siempre?"),
                "Eliminar",
                move || {
                    {
                        let mut st = state_c.borrow_mut();
                        let _ = st.storage.purge_note(&id_c2);
                    }
                    update_search_c();
                    rebuild_trash_rows(&list_c, &state_c, &update_search_c, &confirm_parent);
                },
            );
        });
    }
}

// Fuente única de atajos de ventana (scope de módulo para que los tests la
// vean): esta tabla REGISTRA cada binding (el handler despacha por `action`)
// y a la vez DOCUMENTA el cheatsheet (la ventana se renderiza de acá).
// Agregar un atajo = agregar una fila; el test
// `window_shortcuts_table_is_consistent` impide combos duplicados y acciones
// sin documentar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ShortcutAction {
    NextNote,
    PrevNote,
    FocusSearch,
    NewNote,
    DeleteNote,
    OpenTrash,
    RenameNote,
    ThemePicker,
    OpenShortcuts,
    EscapeContextual,
}

struct WindowShortcut {
    key: Key,
    with_ctrl: bool,
    section: &'static str,
    keys_label: &'static str,
    description: &'static str,
    action: ShortcutAction,
}

const WINDOW_SHORTCUTS: &[WindowShortcut] = &[
    WindowShortcut {
        key: Key::j,
        with_ctrl: true,
        section: "Generales",
        keys_label: "Ctrl+J",
        description: "Nota siguiente",
        action: ShortcutAction::NextNote,
    },
    WindowShortcut {
        key: Key::k,
        with_ctrl: true,
        section: "Generales",
        keys_label: "Ctrl+K",
        description: "Nota anterior",
        action: ShortcutAction::PrevNote,
    },
    WindowShortcut {
        key: Key::l,
        with_ctrl: true,
        section: "Generales",
        keys_label: "Ctrl+L",
        description: "Ir al buscador",
        action: ShortcutAction::FocusSearch,
    },
    WindowShortcut {
        key: Key::f,
        with_ctrl: true,
        section: "Generales",
        keys_label: "Ctrl+F",
        description: "Ir al buscador",
        action: ShortcutAction::FocusSearch,
    },
    WindowShortcut {
        key: Key::n,
        with_ctrl: true,
        section: "Generales",
        keys_label: "Ctrl+N",
        description: "Nota nueva",
        action: ShortcutAction::NewNote,
    },
    WindowShortcut {
        key: Key::d,
        with_ctrl: true,
        section: "Generales",
        keys_label: "Ctrl+D",
        description: "Mover la nota actual a la papelera",
        action: ShortcutAction::DeleteNote,
    },
    WindowShortcut {
        key: Key::t,
        with_ctrl: true,
        section: "Generales",
        keys_label: "Ctrl+T",
        description: "Abrir la papelera",
        action: ShortcutAction::OpenTrash,
    },
    WindowShortcut {
        key: Key::r,
        with_ctrl: true,
        section: "Generales",
        keys_label: "Ctrl+R",
        description: "Renombrar la nota actual",
        action: ShortcutAction::RenameNote,
    },
    WindowShortcut {
        key: Key::p,
        with_ctrl: true,
        section: "Generales",
        keys_label: "Ctrl+P",
        description: "Selector de tema",
        action: ShortcutAction::ThemePicker,
    },
    WindowShortcut {
        key: Key::question,
        with_ctrl: true,
        section: "Generales",
        keys_label: "Ctrl+?",
        description: "Mostrar esta ventana",
        action: ShortcutAction::OpenShortcuts,
    },
    WindowShortcut {
        key: Key::Escape,
        with_ctrl: false,
        section: "Generales",
        keys_label: "Esc",
        description: "Cerrar panel / ir al buscador",
        action: ShortcutAction::EscapeContextual,
    },
];

pub fn build_ui(app: &Application) -> UiHandles {
    let config = Config::load();
    // Aplica el tema guardado antes de construir la ventana para que cada
    // widget lo tome desde el arranque. El handle queda vivo en un `Rc` para
    // que el selector del pie pueda re-aplicar la paleta sin reiniciar.
    let initial_theme = ThemeId::from_persisted(&config.theme);
    let theme_handle: Option<Rc<theme::ThemeHandle>> =
        gtk4::gdk::Display::default().map(|display| Rc::new(theme::load(&display, initial_theme)));
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

    let editor_overlay = Overlay::new();
    editor_overlay.set_child(Some(&text_scroll));
    editor_box.append(&editor_overlay);

    // Panel de resultados para el modo vertical: overlay con la lista de notas
    // mientras el buscador está activo, misma estética que el autocomplete.
    let results_list_box = ListBox::new();
    results_list_box.set_selection_mode(SelectionMode::Single);
    results_list_box.add_css_class("navigation-sidebar");

    let results_scroll = ScrolledWindow::builder()
        .child(&results_list_box)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .max_content_height(400)
        .propagate_natural_height(true)
        .build();

    let results_panel = GtkBox::new(Orientation::Vertical, 0);
    results_panel.append(&results_scroll);
    results_panel.add_css_class("nv-autocomplete-panel");
    results_panel.set_halign(Align::Fill);
    results_panel.set_valign(Align::Start);
    results_panel.set_margin_start(8);
    results_panel.set_margin_end(8);
    results_panel.set_margin_top(4);
    results_panel.set_visible(false);

    editor_overlay.add_overlay(&results_panel);

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

    // Selector de tema en el pie: un MenuButton con los cuatro temas como
    // radio items. La elección se aplica en vivo y se persiste, sin reiniciar.
    let theme_button = MenuButton::new();
    theme_button.set_label(initial_theme.display_name());
    theme_button.set_tooltip_text(Some("Tema (Ctrl+P)"));
    let theme_popover = Popover::new();
    let theme_list = GtkBox::new(Orientation::Vertical, 0);
    theme_popover.set_child(Some(&theme_list));
    theme_button.set_popover(Some(&theme_popover));
    status_box.append(&theme_button);

    const THEME_ORDER: [ThemeId; 4] = [
        ThemeId::Catppuccin,
        ThemeId::Dracula,
        ThemeId::Flexoki,
        ThemeId::Wallpaper,
    ];
    let mut theme_leader: Option<CheckButton> = None;
    for id in THEME_ORDER {
        let item = CheckButton::with_label(id.display_name());
        if let Some(ref leader) = theme_leader {
            item.set_group(Some(leader));
        } else {
            theme_leader = Some(item.clone());
        }
        item.set_active(id == initial_theme);
        let theme_handle_c = theme_handle.clone();
        let state_c = state.clone();
        let theme_button_c = theme_button.clone();
        item.connect_toggled(move |button| {
            if !button.is_active() {
                return;
            }
            if let Some(ref handle) = theme_handle_c {
                handle.apply(id);
            }
            state_c.borrow_mut().config.theme = id.as_str().to_string();
            state_c.borrow().config.save();
            theme_button_c.set_label(id.display_name());
        });
        theme_list.append(&item);
    }

    // Ventana de atajos de teclado: botón en la esquina inferior izquierda
    // del pie + Ctrl+?. Reutiliza el patrón del diálogo de papelera.
    // Tabla de atajos en scope de módulo (arriba): el handler despacha por
    // `action` y el cheatsheet se renderiza de la misma tabla.

    // Atajos del editor (controller propio en wiki_autocomplete.rs, activos
    // solo con sugerencias): contenido estático del cheatsheet.
    const EDITOR_SHORTCUTS: &[(&str, &str)] = &[
        ("Enter / Tab", "Insertar la sugerencia [[ ]]"),
        ("↑ / ↓", "Navegar las sugerencias"),
        ("Ctrl+Enter", "Seguir el enlace bajo el cursor"),
        ("Ctrl+Mayús+Enter", "Crear nota desde la búsqueda"),
        ("Esc", "Cerrar las sugerencias"),
    ];

    fn append_section_header(parent: &GtkBox, title: &str) {
        let header = Label::new(Some(title));
        header.set_halign(Align::Start);
        header.add_css_class("dim-label");
        parent.append(&header);
    }

    fn append_shortcut_row(parent: &GtkBox, keys: &str, description: &str) {
        let row = GtkBox::new(Orientation::Horizontal, 12);
        let keys_label = Label::new(Some(keys));
        keys_label.set_halign(Align::Start);
        keys_label.set_width_chars(20);
        let action_label = Label::new(Some(description));
        action_label.set_halign(Align::Start);
        action_label.set_hexpand(true);
        action_label.set_wrap(true);
        row.append(&keys_label);
        row.append(&action_label);
        parent.append(&row);
    }

    let open_shortcuts: Rc<dyn Fn()> = Rc::new({
        let window = window.clone();
        move || {
            let dialog = Window::builder()
                .transient_for(&window)
                .modal(true)
                .title("Atajos de teclado")
                .default_width(440)
                .default_height(480)
                .build();

            let vbox = GtkBox::new(Orientation::Vertical, 6);
            vbox.set_margin_top(12);
            vbox.set_margin_bottom(12);
            vbox.set_margin_start(12);
            vbox.set_margin_end(12);
            dialog.set_child(Some(&vbox));

            let scroll = ScrolledWindow::builder().vexpand(true).build();
            vbox.append(&scroll);
            let list = GtkBox::new(Orientation::Vertical, 8);
            scroll.set_child(Some(&list));

            // Secciones y filas salen de WINDOW_SHORTCUTS: lo que no está en
            // la tabla no existe como atajo ni como documentación.
            let mut last_section = "";
            for sc in WINDOW_SHORTCUTS {
                if sc.section != last_section {
                    last_section = sc.section;
                    append_section_header(&list, sc.section);
                }
                append_shortcut_row(&list, sc.keys_label, sc.description);
            }
            append_section_header(&list, "Editor");
            for &(keys, description) in EDITOR_SHORTCUTS {
                append_shortcut_row(&list, keys, description);
            }

            let bottom = GtkBox::new(Orientation::Horizontal, 6);
            bottom.set_halign(Align::End);
            let close_btn = Button::with_label("Cerrar");
            bottom.append(&close_btn);
            vbox.append(&bottom);
            {
                let dialog_c = dialog.clone();
                close_btn.connect_clicked(move |_| {
                    dialog_c.close();
                });
            }

            // Keyboard: Esc closes the shortcuts window.
            let esc_controller = EventControllerKey::new();
            let dialog_c = dialog.clone();
            esc_controller.connect_key_pressed(move |_, key, _, _| {
                if key == Key::Escape {
                    dialog_c.close();
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            });
            dialog.add_controller(esc_controller);

            dialog.present();
        }
    });
    let shortcuts_button = Button::from_icon_name("input-keyboard-symbolic");
    shortcuts_button.set_tooltip_text(Some("Atajos de teclado (Ctrl+?)"));
    {
        let open_shortcuts_c = Rc::clone(&open_shortcuts);
        shortcuts_button.connect_clicked(move |_| open_shortcuts_c());
    }
    status_box.prepend(&shortcuts_button);

    editor_box.append(&status_box);

    paned.set_end_child(Some(&editor_box));
    window.set_content(Some(&paned));

    // ── Responsive layout ─────────────────────────────────────────────
    // Estado compartido del modo vertical (ventana más alta que ancha).
    let is_portrait = Rc::new(Cell::new(false));

    // Muestra/oculta el overlay de resultados de búsqueda del modo vertical.
    let set_results_visible = {
        let results_panel = results_panel.clone();
        move |visible: bool| results_panel.set_visible(visible)
    };

    // Regla de visibilidad del overlay de resultados en modo vertical: visible
    // cuando el buscador tiene foco o hay una query activa. Se expone como
    // Rc<dyn Fn> para que los tests apliquen la regla sin depender del foco X.
    let update_results_visibility: Rc<dyn Fn()> = Rc::new({
        let search_entry = search_entry.clone();
        let set_results_visible = set_results_visible.clone();
        let is_portrait = Rc::clone(&is_portrait);

        move || {
            let focused = search_entry.has_focus();
            let query = search_entry.text().to_string();
            let should_show = results_overlay_visible(is_portrait.get(), focused, &query);
            set_results_visible(should_show);
        }
    });

    // Aplica el layout según la orientación real de la ventana. En vertical el
    // buscador queda arriba de todo y la lista de notas queda oculta; en
    // horizontal se restaura el split clásico de dos paneles. Se expone como
    // Rc<dyn Fn> para que los tests lo disparen sin depender del frame clock.
    let apply_layout: Rc<dyn Fn(bool)> = Rc::new({
        let paned = paned.clone();
        let list_scroll = list_scroll.clone();
        let left_box = left_box.clone();
        let is_portrait = Rc::clone(&is_portrait);
        let update_results_visibility = update_results_visibility.clone();

        move |portrait: bool| {
            if is_portrait.get() == portrait {
                return;
            }
            is_portrait.set(portrait);

            if portrait {
                paned.set_orientation(Orientation::Vertical);
                list_scroll.set_visible(false);
                // El pane superior contiene la barra de búsqueda: posicionar el
                // divisor en su altura natural (con la lista oculta) para que la
                // barra quede SIEMPRE visible arriba. set_position(0) la colapsaba.
                let (_, natural) = left_box.preferred_size();
                paned.set_position(natural.height().max(48) as i32);
            } else {
                paned.set_orientation(Orientation::Horizontal);
                paned.set_position(300);
                list_scroll.set_visible(true);
            }

            update_results_visibility();
        }
    });

    // La lista "activa" depende del modo: en vertical los resultados viven en el
    // overlay; en horizontal, en la sidebar clásica.
    let active_list_box = {
        let list_box = list_box.clone();
        let results_list_box = results_list_box.clone();
        let is_portrait = Rc::clone(&is_portrait);

        move || {
            if is_portrait.get() {
                results_list_box.clone()
            } else {
                list_box.clone()
            }
        }
    };

    // Detecta la orientación en cada re-alocación: height > width = vertical.
    // Se usa el tick callback porque el signal "size-allocate" no está expuesto
    // como connect_* en gtk4-rs 0.9.
    window.add_tick_callback({
        let apply_layout = apply_layout.clone();
        move |win, _frame_clock| {
            let portrait = win.height() > win.width();
            apply_layout(portrait);
            glib::ControlFlow::Continue
        }
    });

    // Helper functions for UI refresh
    // Construye una fila de lista para una nota; la comparten la sidebar y el
    // overlay de resultados del modo vertical.
    let build_note_row = move |note: &nv_core::note::Note| -> ListBoxRow {
        let row_box = GtkBox::new(Orientation::Vertical, 2);
        row_box.set_margin_start(10);
        row_box.set_margin_end(10);
        row_box.set_margin_top(6);
        row_box.set_margin_bottom(6);

        let display_title = note
            .content
            .lines()
            .map(|l| l.trim())
            .find(|l| !l.is_empty())
            .unwrap_or("(nota vacía)")
            .to_string();

        let title_label = Label::new(Some(&display_title));
        title_label.set_halign(Align::Fill);
        title_label.set_xalign(0.0);
        title_label.add_css_class("heading");
        title_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        title_label.set_max_width_chars(1);
        title_label.set_hexpand(true);
        title_label.set_tooltip_text(Some(&display_title));

        let meta_str = format!(
            "{} (creada {}) • {}",
            note.formatted_date(),
            note.formatted_created_date(),
            note.tags.join(" ")
        );
        let meta_label = Label::new(Some(&meta_str));
        meta_label.set_halign(Align::Fill);
        meta_label.set_xalign(0.0);
        meta_label.add_css_class("caption");
        meta_label.add_css_class("dim-label");
        meta_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        meta_label.set_max_width_chars(1);
        meta_label.set_hexpand(true);
        meta_label.set_tooltip_text(Some(&meta_str));

        row_box.append(&title_label);
        row_box.append(&meta_label);

        let row = ListBoxRow::new();
        row.set_child(Some(&row_box));
        row
    };

    // Reconstruye la lista desde el estado REAL de storage (NOTA: usa `filtered_indices` ya
    // derivadas correctamente). Se llama tras un save para que el sidebar refleje el nuevo
    // orden del re-sort — de lo contrario navegar por índice (Ctrl+J/K) tras guardar puede
    // abrir la nota equivocada.
    let populate_list = {
        let state = Rc::clone(&state);
        let list_box = list_box.clone();
        let results_list_box = results_list_box.clone();
        let status_label = status_label.clone();
        let search_entry = search_entry.clone();
        let build_note_row = build_note_row.clone();

        move || {
            // Remove all rows from both lists (sidebar + portrait overlay)
            while let Some(child) = list_box.first_child() {
                list_box.remove(&child);
            }
            while let Some(child) = results_list_box.first_child() {
                results_list_box.remove(&child);
            }

            let st = state.borrow();
            let query = search_entry.text().to_string();

            for id in &st.filtered_indices {
                if let Some(note) = st.storage.get_note(id) {
                    list_box.append(&build_note_row(note));
                    results_list_box.append(&build_note_row(note));
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

    // Re-selecciona la fila que corresponde a un id en la lista, SIN tocar el editor.
    // Se llama tras reconciliar (p. ej. un re-sort por modified_at tras save_note) para
    // restaurar la selección visual por índice sin disparar el echo del buffer.
    let select_row_by_id = {
        let state = Rc::clone(&state);
        let active_list_box = active_list_box.clone();

        move |target_id: &str| {
            let lb = active_list_box();
            let st = state.borrow();
            if let Some(pos) = st.filtered_indices.iter().position(|id| id == target_id) {
                if let Some(row) = lb.row_at_index(pos as i32) {
                    drop(st);
                    lb.select_row(Some(&row));
                }
            }
        }
    };

    // Re-concilia la lista tras un guardado (debounced o flush). El re-sort de notas por
    // modified_at (storage.save_note) no toca `filtered_indices`, así que la barra lateral
    // queda con orden viejo y la navegación por índice (Ctrl+J/K) puede abrir la nota
    // equivocada. Aquí re-derivamos el filtro respetando la query activa y re-poblamos,
    // restaurando la selección SIN tocar el editor (nada de echo del buffer).

    let reconcile_after_edit = {
        let state = Rc::clone(&state);
        let search_entry = search_entry.clone();
        let populate_list = populate_list.clone();
        let select_row_by_id = select_row_by_id.clone();

        move || {
            let query = search_entry.text().to_string();
            {
                let mut st = state.borrow_mut();
                st.filtered_indices = search_notes(&st.storage.notes, &query);
            }

            populate_list();

            let current_id = {
                let st = state.borrow();
                st.current_note_id.clone()
            };
            if let Some(id) = current_id {
                select_row_by_id(&id);
            }
        }
    };

    // Reconstruye la lista desde el estado real de storage (NOTA: usa `notes` ya ordenadas
    // por el re-sort de `save_note`). Se llama después de guardar para que el sidebar y
    // `filtered_indices` reflejen el nuevo orden — de lo contrario la selección por índice
    // (Ctrl+J/K y el click) navega a la nota equivocada tras un re-sort.
    let reconcile_after_save = {
        let state = Rc::clone(&state);
        let search_entry = search_entry.clone();
        let populate_list = populate_list.clone();
        let select_row_by_id = select_row_by_id.clone();

        move || {
            let query = search_entry.text().to_string();
            {
                let mut st = state.borrow_mut();
                st.filtered_indices = search_notes(&st.storage.notes, &query);
            }

            populate_list();

            let current_id = state.borrow().current_note_id.clone();
            if let Some(id) = current_id {
                select_row_by_id(&id);
            }
        }
    };

    // Al hacer Ctrl+Enter sobre un wiki-link, pone su texto en la barra de búsqueda y filtra
    let search_wiki_target = {
        let state = Rc::clone(&state);
        let populate_list = populate_list.clone();
        let search_entry = search_entry.clone();

        move |target: &str| {
            let target_clean = target.trim();
            if target_clean.is_empty() {
                return;
            }

            search_entry.set_text(target_clean);
            search_entry.grab_focus();
            search_entry.select_region(0, -1);

            {
                let mut st = state.borrow_mut();
                st.filtered_indices = search_notes(&st.storage.notes, target_clean);
            }
            populate_list();
        }
    };

    // Sistema de wiki-links: resaltado + autocompletado difuso con panel flotante
    let wiki = WikiAutocomplete::setup(Rc::clone(&state), text_view.clone(), editor_overlay, {
        let search_wiki_target = search_wiki_target.clone();
        move |target: &str| search_wiki_target(target)
    });

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
                    drop(buffer);

                    let _ = st.storage.save_note(&current_id, &text);
                    drop(st);
                    reconcile_after_edit();
                    reconcile_after_save();
                }
            }
        }
    };

    let select_note_by_id = {
        let state = Rc::clone(&state);
        let text_view = text_view.clone();
        let active_list_box = active_list_box.clone();
        let info_label = info_label.clone();
        let flush_pending_save = flush_pending_save.clone();
        let wiki = wiki.clone();

        move |target_id: &str| {
            wiki.close();
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
                    let lb = active_list_box();
                    if let Some(row) = lb.row_at_index(pos as i32) {
                        lb.select_row(Some(&row));
                    }
                }
            }

            {
                let mut st = state.borrow_mut();
                st.is_updating_ui = false;
            }

            wiki.refresh();

            // Mover el foco al editor con el cursor al final del texto
            let buffer = text_view.buffer();
            let end_iter = buffer.end_iter();
            buffer.place_cursor(&end_iter);
            text_view.grab_focus();
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
        let update_results_visibility = update_results_visibility.clone();
        move |_| {
            update_search();
            update_results_visibility();
        }
    });

    // Al entrar/salir el foco del buscador se actualiza el overlay de resultados
    // del modo vertical (visible mientras está enfocado o hay query activa).
    search_entry.connect_notify_local(Some("has-focus"), {
        let update_results_visibility = update_results_visibility.clone();
        move |_se, _spec| {
            update_results_visibility();
        }
    });

    // Connect Search Entry Activate (Enter key in search bar)
    search_entry.connect_activate({
        let state = Rc::clone(&state);
        let search_entry = search_entry.clone();
        let text_view = text_view.clone();
        let populate_list = populate_list.clone();
        let select_note_by_id = select_note_by_id.clone();
        let set_results_visible = set_results_visible.clone();

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
                    match st.storage.create_note(&timestamp_title()) {
                        Ok(new_note) => {
                            let new_id = new_note.id.clone();
                            let _ = st.storage.save_note(&new_id, query_clean);
                            st.filtered_indices =
                                st.storage.notes.iter().map(|n| n.id.clone()).collect();
                            (true, new_id)
                        }
                        // Disk failure: no note to select, keep the query as-is.
                        Err(_) => (false, String::new()),
                    }
                }
            };

            let (should_select, id) = action;
            if should_select {
                populate_list();
                select_note_by_id(&id);
            }
            text_view.grab_focus();
            // En vertical, Enter cierra el overlay de resultados.
            set_results_visible(false);
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

    // Al activar una fila del overlay de resultados (modo vertical) se abre la
    // nota y se cierra el overlay.
    results_list_box.connect_row_activated({
        let state = Rc::clone(&state);
        let select_note_by_id = select_note_by_id.clone();
        let set_results_visible = set_results_visible.clone();
        let is_portrait = Rc::clone(&is_portrait);

        move |_, row| {
            let idx = row.index() as usize;
            let target_id = {
                let st = state.borrow();
                st.filtered_indices.get(idx).cloned()
            };

            if let Some(id) = target_id {
                select_note_by_id(&id);
            }
            if is_portrait.get() {
                set_results_visible(false);
            }
        }
    });

    // Auto-Save when editing TextBuffer (debounced usando config.auto_save_ms)
    text_view.buffer().connect_changed({
        let state = Rc::clone(&state);
        let info_label = info_label.clone();
        let wiki = wiki.clone();

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
                        let _ = st.storage.save_note(&current_id, &text);
                        st.save_timeout_source = None;

                        glib::ControlFlow::Break // ejecutar una sola vez
                    },
                );

                st.save_timeout_source = Some(source_id);
            }

            drop(st);
            wiki.refresh();
            wiki.update();
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
            let Ok(note) = st.storage.create_note(&timestamp_title()) else {
                return;
            };
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
        let text_view = text_view.clone();
        let info_label = info_label.clone();

        move || {
            let mut st = state.borrow_mut();
            if let Some(id) = st.current_note_id.clone() {
                let _ = st.storage.delete_note(&id);
                st.current_note_id = None;
                st.current_wiki_links = Vec::new();
                drop(st);

                search_entry.set_text("");
                update_search();

                // Limpiar el editor: no dejar el contenido de la nota borrada en
                // pantalla. Sin esto, el buffer muestra un "fantasma" y, como ya
                // no hay nota seleccionada, lo que el usuario escriba se pierde.
                {
                    let buffer = text_view.buffer();
                    // No mantener el guard de `borrow_mut()` vivo durante `set_text`:
                    // emite `changed` sincrónicamente y su handler pide otro
                    // `borrow_mut()` -> panic "RefCell already borrowed" (Ctrl+D).
                    // Además el flag tiene que volver a false (antes quedaba en true
                    // y el auto-save quedaba suprimido tras borrar).
                    state.borrow_mut().is_updating_ui = true;
                    buffer.set_text("");
                    state.borrow_mut().is_updating_ui = false;
                }

                info_label.set_text("Nota movida a la papelera");
                text_view.grab_focus();
            }
        }
    };

    // Rename Note dialog: type the new title — Ctrl+R
    let rename_current_note = {
        let state = Rc::clone(&state);
        let window = window.clone();
        let update_search = update_search.clone();
        let flush_pending_save = flush_pending_save.clone();
        let select_note_by_id = select_note_by_id.clone();

        move || {
            let current_id: Option<String> = state.borrow().current_note_id.clone();
            let Some(id) = current_id else {
                return;
            };
            flush_pending_save();

            let dialog = Window::builder()
                .transient_for(&window)
                .modal(true)
                .title("Renombrar nota")
                .default_width(360)
                .build();
            let vbox = GtkBox::new(Orientation::Vertical, 8);
            vbox.set_margin_top(12);
            vbox.set_margin_bottom(12);
            vbox.set_margin_start(12);
            vbox.set_margin_end(12);
            dialog.set_child(Some(&vbox));
            let entry = Entry::builder().text(&id).build();
            entry.select_region(0, -1);
            vbox.append(&entry);
            let buttons = GtkBox::new(Orientation::Horizontal, 6);
            buttons.set_halign(Align::End);
            let ok_btn = Button::with_label("Renombrar");
            let cancel_btn = Button::with_label("Cancelar");
            buttons.append(&cancel_btn);
            buttons.append(&ok_btn);
            vbox.append(&buttons);

            let accept = {
                let state = Rc::clone(&state);
                let entry = entry.clone();
                let dialog = dialog.clone();
                let update_search = update_search.clone();
                let select_note_by_id = select_note_by_id.clone();
                move || {
                    let new_title = entry.text().to_string();
                    let new_id: Option<String> = {
                        let mut st = state.borrow_mut();
                        match st.storage.rename_note(&id, &new_title) {
                            Ok(Some(note)) => Some(note.id.clone()),
                            _ => None,
                        }
                    };
                    update_search();
                    if let Some(new_id) = new_id {
                        select_note_by_id(&new_id);
                    }
                    dialog.close();
                }
            };
            let accept_c = accept.clone();
            ok_btn.connect_clicked(move |_| accept());
            entry.connect_activate(move |_| accept_c());
            let dialog_c = dialog.clone();
            cancel_btn.connect_clicked(move |_| dialog_c.close());

            // Keyboard: Enter confirms via the entry, Esc closes.
            let esc_controller = EventControllerKey::new();
            let dialog_c = dialog.clone();
            esc_controller.connect_key_pressed(move |_, key, _, _| {
                if key == Key::Escape {
                    dialog_c.close();
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            });
            dialog.add_controller(esc_controller);

            dialog.present();
            entry.grab_focus();
        }
    };

    // Papelera: diálogo con las notas borradas (restaurar / eliminar
    // definitivo por fila + vaciar). Se abre con Ctrl+T.
    let open_trash_dialog = {
        let state = Rc::clone(&state);
        let window = window.clone();
        let update_search = update_search.clone();

        move || {
            let update_search: Rc<dyn Fn()> = Rc::new(update_search.clone());
            let dialog = Window::builder()
                .transient_for(&window)
                .modal(true)
                .title("Papelera")
                .default_width(420)
                .default_height(320)
                .build();

            let vbox = GtkBox::new(Orientation::Vertical, 6);
            vbox.set_margin_top(12);
            vbox.set_margin_bottom(12);
            vbox.set_margin_start(12);
            vbox.set_margin_end(12);
            dialog.set_child(Some(&vbox));

            let scroll = ScrolledWindow::builder().vexpand(true).build();
            vbox.append(&scroll);
            let list = ListBox::new();
            list.set_selection_mode(SelectionMode::None);
            scroll.set_child(Some(&list));

            let refresh = {
                let list = list.clone();
                let state = Rc::clone(&state);
                let update_search = update_search.clone();
                let dialog = dialog.clone();
                move || rebuild_trash_rows(&list, &state, &update_search, &dialog)
            };

            let bottom = GtkBox::new(Orientation::Horizontal, 6);
            bottom.set_halign(Align::End);
            let empty_btn = Button::with_label("Vaciar papelera");
            let close_btn = Button::with_label("Cerrar");
            bottom.append(&empty_btn);
            bottom.append(&close_btn);
            vbox.append(&bottom);

            {
                let list_c = list.clone();
                let state_c = Rc::clone(&state);
                let update_search_c = update_search.clone();
                let dialog_c = dialog.clone();
                empty_btn.connect_clicked(move |_| {
                    let trashed_count = state_c.borrow().storage.trash_notes().len();
                    if trashed_count == 0 {
                        return;
                    }
                    let list_c = list_c.clone();
                    let state_c = Rc::clone(&state_c);
                    let update_search_c = update_search_c.clone();
                    let dialog_c2 = dialog_c.clone();
                    confirm_dialog(
                        &dialog_c,
                        "Vaciar papelera",
                        &format!(
                            "¿Eliminar para siempre las {trashed_count} notas de la papelera?"
                        ),
                        "Vaciar",
                        move || {
                            {
                                let mut st = state_c.borrow_mut();
                                let _ = st.storage.empty_trash();
                            }
                            update_search_c();
                            rebuild_trash_rows(&list_c, &state_c, &update_search_c, &dialog_c2);
                        },
                    );
                });
            }
            {
                let dialog_c = dialog.clone();
                close_btn.connect_clicked(move |_| {
                    dialog_c.close();
                });
            }

            // Keyboard: Esc closes the trash dialog.
            let esc_controller = EventControllerKey::new();
            let dialog_c = dialog.clone();
            esc_controller.connect_key_pressed(move |_, key, _, _| {
                if key == Key::Escape {
                    dialog_c.close();
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            });
            dialog.add_controller(esc_controller);

            refresh();
            dialog.present();
        }
    };

    // Mueve la selección de la lista de notas hacia abajo (+1) o arriba (-1) y la abre
    let move_list_selection = {
        let state = Rc::clone(&state);
        let active_list_box = active_list_box.clone();
        let select_note_by_id = select_note_by_id.clone();
        let is_portrait = Rc::clone(&is_portrait);
        let set_results_visible = set_results_visible.clone();

        move |delta: i32| {
            let lb = active_list_box();

            // En vertical, navegar con J/K muestra el overlay si estaba oculto.
            if is_portrait.get() && !lb.is_visible() {
                set_results_visible(true);
            }

            let current_index = lb.selected_row().map(|row| row.index()).unwrap_or(-1);

            let target_index = current_index + delta;
            if target_index < 0 {
                return;
            }

            if let Some(row) = lb.row_at_index(target_index) {
                lb.select_row(Some(&row));
                row.grab_focus();

                let idx = target_index as usize;
                let target_id = {
                    let st = state.borrow();
                    st.filtered_indices.get(idx).cloned()
                };
                if let Some(id) = target_id {
                    select_note_by_id(&id);
                }

                // Al abrir la nota el foco pasa al editor y la regla de visibilidad
                // podría ocultar el overlay (query vacía); lo mantenemos visible
                // para poder seguir navegando con J/K.
                if is_portrait.get() {
                    set_results_visible(true);
                }
            }
        }
    };

    // Keyboard Controller for Global App Shortcuts (Ctrl+L, Esc, Ctrl+N, Ctrl+D, Ctrl+T, Ctrl+R, Ctrl+J, Ctrl+K, Ctrl+P)
    let key_controller = EventControllerKey::new();
    key_controller.connect_key_pressed({
        let search_entry = search_entry.clone();
        let _list_box = list_box.clone();
        let text_view = text_view.clone();
        let results_panel = results_panel.clone();
        let is_portrait = Rc::clone(&is_portrait);
        let set_results_visible = set_results_visible.clone();
        let create_new_empty_note = create_new_empty_note.clone();
        let delete_current_note = delete_current_note.clone();
        let open_trash_dialog = open_trash_dialog.clone();
        let rename_current_note = rename_current_note.clone();
        let move_list_selection = move_list_selection.clone();
        let open_shortcuts = Rc::clone(&open_shortcuts);
        let theme_button = theme_button.clone();

        move |_, key, _, modifier| {
            let is_ctrl = modifier.contains(gdk::ModifierType::CONTROL_MASK);

            let Some(hit) = WINDOW_SHORTCUTS
                .iter()
                .find(|s| s.key == key && s.with_ctrl == is_ctrl)
            else {
                return glib::Propagation::Proceed;
            };
            match hit.action {
                ShortcutAction::NextNote => move_list_selection(1),
                ShortcutAction::PrevNote => move_list_selection(-1),
                ShortcutAction::FocusSearch => {
                    search_entry.grab_focus();
                    search_entry.select_region(0, -1);
                }
                ShortcutAction::NewNote => create_new_empty_note(),
                ShortcutAction::DeleteNote => delete_current_note(),
                ShortcutAction::OpenTrash => open_trash_dialog(),
                ShortcutAction::RenameNote => rename_current_note(),
                ShortcutAction::ThemePicker => theme_button.popup(),
                ShortcutAction::OpenShortcuts => open_shortcuts(),
                ShortcutAction::EscapeContextual => {
                    // En vertical, Esc cierra el overlay de resultados si está
                    // visible; si no, enfoca el buscador (comportamiento previo).
                    if is_portrait.get() && results_panel.is_visible() {
                        set_results_visible(false);
                        text_view.grab_focus();
                    } else {
                        search_entry.grab_focus();
                        search_entry.select_region(0, -1);
                    }
                }
            }
            glib::Propagation::Stop
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

    UiHandles {
        window: window.clone(),
        paned: paned.clone(),
        list_scroll: list_scroll.clone(),
        search_entry: search_entry.clone(),
        list_box: list_box.clone(),
        results_panel: results_panel.clone(),
        results_list_box: results_list_box.clone(),
        text_view: text_view.clone(),
        is_portrait: Rc::clone(&is_portrait),
        apply_layout,
        update_results_visibility,
    }
}

/// Regla de visibilidad del overlay de resultados en modo vertical: se muestra
/// cuando la ventana es vertical y el buscador tiene foco o hay una query
/// activa (no vacía al recortar espacios).
fn results_overlay_visible(portrait: bool, focused: bool, query: &str) -> bool {
    portrait && (focused || !query.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Procesa eventos pendientes del main loop de GTK varias rondas.
    fn pump(rounds: usize) {
        let ctx = gtk4::glib::MainContext::default();
        for _ in 0..rounds {
            while ctx.pending() {
                ctx.iteration(false);
            }
        }
    }

    #[test]
    fn results_overlay_visibility_rule() {
        // Sin orientación vertical el overlay nunca se muestra.
        for focused in [false, true] {
            for q in ["", "proy", "   ", " x "] {
                assert!(!results_overlay_visible(false, focused, q));
            }
        }
        // Vertical: foco o query no-vacía lo muestran; sin foco y query vacía, no.
        assert!(!results_overlay_visible(true, false, ""));
        assert!(!results_overlay_visible(true, false, "   "));
        assert!(results_overlay_visible(true, true, ""));
        assert!(results_overlay_visible(true, true, "proy"));
        assert!(results_overlay_visible(true, false, "proy"));
        assert!(results_overlay_visible(true, false, "  x"));
    }

    #[test]
    fn window_shortcuts_table_is_consistent() {
        // Sin combos duplicados: dos filas nunca pueden reclamar la misma tecla.
        for (i, a) in WINDOW_SHORTCUTS.iter().enumerate() {
            for b in &WINDOW_SHORTCUTS[i + 1..] {
                assert!(
                    !(a.key == b.key && a.with_ctrl == b.with_ctrl),
                    "atajo duplicado en WINDOW_SHORTCUTS"
                );
            }
        }
        // Toda acción tiene al menos una fila: lo que se ejecuta se documenta.
        // (Al agregar una variante a ShortcutAction, agregarla también acá.)
        for action in [
            ShortcutAction::NextNote,
            ShortcutAction::PrevNote,
            ShortcutAction::FocusSearch,
            ShortcutAction::NewNote,
            ShortcutAction::DeleteNote,
            ShortcutAction::OpenTrash,
            ShortcutAction::RenameNote,
            ShortcutAction::ThemePicker,
            ShortcutAction::OpenShortcuts,
            ShortcutAction::EscapeContextual,
        ] {
            assert!(
                WINDOW_SHORTCUTS.iter().any(|s| s.action == action),
                "acción sin fila en WINDOW_SHORTCUTS: {action:?}"
            );
        }
        // Filas con contenido para el cheatsheet.
        for sc in WINDOW_SHORTCUTS {
            assert!(
                !sc.section.is_empty() && !sc.keys_label.is_empty() && !sc.description.is_empty()
            );
        }
    }

    // Requiere display: `xvfb-run -a cargo test -- --test-threads=1`
    // La orientación la dispara el tick callback (probado por smoke: al
    // redimensionar 500x800 el log muestra `apply_layout -> portrait=true`);
    // aquí se usa el seam expuesto para probar el layout de forma determinista.
    #[test]
    fn responsive_portrait_layout_and_overlay() {
        // Requiere un display de verdad: se corre con
        // `xvfb-run -a cargo test -- --test-threads=1`. Sin display el test se
        // omite; la regla pura queda cubierta por `results_overlay_visibility_rule`.
        gtk4::init().expect("gtk init");
        if gdk::Display::default().is_none() {
            eprintln!("skipping responsive test: no display available");
            return;
        }

        let app = Application::builder()
            .application_id("org.notational.velocity.responsive-test")
            .build();
        let handles = build_ui(&app);
        pump(60);

        // Estado inicial: split horizontal clásico sin overlay.
        assert_eq!(handles.paned.orientation(), Orientation::Horizontal);
        assert!(handles.list_scroll.is_visible());
        assert!(!handles.results_panel.is_visible());

        // Modo vertical: el paned rota y la lista queda oculta.
        (handles.apply_layout)(true);
        pump(40);
        assert_eq!(handles.paned.orientation(), Orientation::Vertical);
        assert!(!handles.list_scroll.is_visible());
        assert!(handles.is_portrait.get());
        // La barra de búsqueda vive en el pane superior: el divisor debe quedar
        // en su altura natural (regresión: set_position(0) la colapsaba).
        assert!(
            handles.paned.position() >= 32,
            "la barra de búsqueda debe quedar visible arriba (position={})",
            handles.paned.position()
        );

        // Una query activa dispara el flujo de búsqueda y muestra el overlay.
        // En el test headless no hay foco X ni se puede sintetizar typing real
        // (set_text no emite search-changed en GTK4), así que se aplica la
        // misma regla que ejecuta el handler de search-changed: con la query
        // real ya en el entry y portrait=true, el overlay debe quedar visible.
        handles.search_entry.set_text("proy");
        pump(40);
        assert_eq!(handles.search_entry.text().to_string(), "proy");
        (handles.update_results_visibility)();
        pump(40);
        assert!(
            handles.results_panel.is_visible(),
            "el overlay debe mostrarse con query activa en modo vertical"
        );

        // Volver a horizontal restaura el split clásico.
        (handles.apply_layout)(false);
        (handles.update_results_visibility)();
        pump(40);
        assert_eq!(handles.paned.orientation(), Orientation::Horizontal);
        assert!(handles.list_scroll.is_visible());
        assert!(!handles.results_panel.is_visible());
        assert!(!handles.is_portrait.get());
    }
}
