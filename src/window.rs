use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use gtk4::gdk::{self, Key};
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{
    Align, Box as GtkBox, Button, CheckButton, Entry, EventControllerKey, Label,
    ListBox, ListBoxRow, MenuButton, Orientation, Overlay, Paned, Popover, ScrolledWindow,
    SearchEntry, SelectionMode, Stack, TextView, Window,
};
use libadwaita::prelude::*;
use libadwaita::{Application, ApplicationWindow};

use crate::app_state::AppState;
use crate::palettes::{ThemeId, Variant, palette};
use crate::theme;
use crate::wiki_autocomplete::{WikiAutocomplete, wiki_link_accent};
use nv_core::config::Config;
use nv_core::search::search_notes;
use nv_core::storage::StorageManager;
use nv_core::util::timestamp_title;
use nv_core::wiki::display_title_or_fallback;

/// Sticky editor title: first non-blank content line, live from the buffer.
/// Reuses the shared `nv_core::wiki` helper (same fallback `(nota vacía)`
/// as the sidebar rows) so desktop shows one consistent title everywhere.
/// Pure — unit-tested below without a display server.
fn sticky_title_for(content: &str) -> String {
    display_title_or_fallback(content)
}

/// Estilo inline de un tramo de markdown renderizado. Salida pura del
/// parser (`markdown_spans`, sin GTK) para poder testearla sin display;
/// `render_markdown_to_buffer` los aplica como TextTags sobre el buffer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum MdStyle {
    H1,
    H2,
    H3,
    Bold,
    Italic,
    Code,
    Link,
    WikiLink,
}

/// Tramo de texto con sus estilos activos. El texto ya viene segmentado por
/// bloque (los `\n` de separación son spans propios sin estilo).
/// `link_url` guarda el destino http(s) del markdown-link que originó el
/// tramo (texto + sufijo ` (url)`); `wiki_target` guarda el texto dentro de
/// `[[...]]`. Ambos son `None` en texto normal: solo los usa la preview
/// para el hover/click, los tests miran `text`/`styles` como siempre.
#[derive(Clone, PartialEq, Eq, Debug)]
struct RichSpan {
    text: String,
    styles: Vec<MdStyle>,
    link_url: Option<String>,
    wiki_target: Option<String>,
}

/// Rango clickeable de la preview ya renderizada (offsets en caracteres
/// sobre el buffer). Exactamente uno de `url`/`wiki` es `Some`.
#[derive(Clone, PartialEq, Eq, Debug)]
struct PreviewLink {
    start: i32,
    end: i32,
    url: Option<String>,
    wiki: Option<String>,
}

fn md_tag_name(style: MdStyle) -> &'static str {
    match style {
        MdStyle::H1 => "md-h1",
        MdStyle::H2 => "md-h2",
        MdStyle::H3 => "md-h3",
        MdStyle::Bold => "md-bold",
        MdStyle::Italic => "md-italic",
        MdStyle::Code => "md-code",
        MdStyle::Link => "md-link",
        MdStyle::WikiLink => "md-wikilink",
    }
}

/// Fondo sutil para el código inline/bloque de la preview: el slot
/// `surface0` de la paleta activa para (`theme`, `dark`), el mismo patrón
/// que [`wiki_link_accent`](crate::wiki_autocomplete::wiki_link_accent)
/// (que aporta el `mauve` para headings/links). Pura, sin GTK.
///
/// `surface0` queda por encima de `base` en claro y oscuro con contraste
/// suficiente para texto normal encima (es el escalón de elevación que el
/// propio stylesheet usa para superficies).
fn preview_code_background(theme: ThemeId, dark: bool) -> String {
    let variant = if dark { Variant::Dark } else { Variant::Light };
    palette(theme, variant).surface0.into_owned()
}

/// Parsea markdown (CommonMark vía `pulldown-cmark`) a tramos con estilo.
/// Subset garantizado: headings, bold, italic, code inline/bloque, listas
/// (bullet + ordenadas), links como `texto (url)`, reglas horizontales.
/// Pura — testeable sin display.
fn markdown_spans(text: &str) -> Vec<RichSpan> {
    use pulldown_cmark::{Event, Parser, Tag, TagEnd};
    let mut spans: Vec<RichSpan> = Vec::new();
    let mut styles: Vec<MdStyle> = Vec::new();
    // Pila de listas: (ordenada, próximo número).
    let mut lists: Vec<(bool, u64)> = Vec::new();
    // Destinos de links/imágenes pendientes (uno por Start anidado).
    let mut link_dests: Vec<String> = Vec::new();

    // El link vigente es el destino del Start más interno todavía abierto.
    let push = |spans: &mut Vec<RichSpan>, s: &str, styles: &[MdStyle], link: Option<&str>| {
        if !s.is_empty() {
            spans.push(RichSpan {
                text: s.to_string(),
                styles: styles.to_vec(),
                link_url: link.map(|l| l.to_string()),
                wiki_target: None,
            });
        }
    };

    for event in Parser::new(text) {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    use pulldown_cmark::HeadingLevel::*;
                    styles.push(match level {
                        H1 => MdStyle::H1,
                        H2 => MdStyle::H2,
                        _ => MdStyle::H3,
                    });
                }
                Tag::Strong => styles.push(MdStyle::Bold),
                Tag::Emphasis => styles.push(MdStyle::Italic),
                Tag::CodeBlock(_) => styles.push(MdStyle::Code),
                Tag::List(first) => {
                    lists.push((first.is_some(), first.unwrap_or(1)));
                }
                Tag::Item => {
                    let depth = lists.len().max(1);
                    let indent = "  ".repeat(depth - 1);
                    let bullet = match lists.last_mut() {
                        Some((true, n)) => {
                            let b = format!("{n}. ");
                            *n += 1;
                            b
                        }
                        _ => "• ".to_string(),
                    };
                    push(&mut spans, &format!("{indent}{bullet}"), &styles, None);
                }
                Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. } => {
                    styles.push(MdStyle::Link);
                    link_dests.push(dest_url.to_string());
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    styles.retain(|s| !matches!(s, MdStyle::H1 | MdStyle::H2 | MdStyle::H3));
                    push(&mut spans, "\n\n", &[], None);
                }
                TagEnd::Paragraph => push(&mut spans, "\n\n", &[], None),
                TagEnd::CodeBlock => {
                    styles.retain(|s| *s != MdStyle::Code);
                    push(&mut spans, "\n", &[], None);
                }
                TagEnd::Item => push(&mut spans, "\n", &[], None),
                TagEnd::List(_) => {
                    lists.pop();
                    push(&mut spans, "\n", &[], None);
                }
                TagEnd::Link | TagEnd::Image => {
                    styles.retain(|s| *s != MdStyle::Link);
                    if let Some(dest) = link_dests.pop() {
                        push(&mut spans, &format!(" ({dest})"), &[MdStyle::Link], Some(&dest));
                    }
                }
                TagEnd::BlockQuote(_) => push(&mut spans, "\n\n", &[], None),
                TagEnd::Table => push(&mut spans, "\n", &[], None),
                TagEnd::TableHead | TagEnd::TableRow => push(&mut spans, "\n", &[], None),
                TagEnd::TableCell => push(&mut spans, " | ", &[], None),
                _ => {}
            },
            Event::Text(t) => {
                let link = link_dests.last().map(|s| s.as_str());
                push(&mut spans, &t, &styles, link);
            }
            Event::Code(c) => {
                let mut with_code = styles.clone();
                if !with_code.contains(&MdStyle::Code) {
                    with_code.push(MdStyle::Code);
                }
                let link = link_dests.last().map(|s| s.as_str());
                push(&mut spans, &c, &with_code, link);
            }
            Event::SoftBreak | Event::HardBreak => push(&mut spans, "\n", &[], None),
            Event::Rule => push(&mut spans, "───\n\n", &[], None),
            Event::Html(_) | Event::InlineHtml(_) | Event::FootnoteReference(_) => {}
            Event::TaskListMarker(_) => {}
            // Eventos de metadata/tablas que no aportan texto visible.
            _ => {}
        }
    }
    // Recortar newlines colgantes del final (el último bloque no necesita
    // separación posterior).
    while let Some(last) = spans.last_mut() {
        let trimmed = last.text.trim_end_matches('\n').to_string();
        if trimmed.is_empty() && last.styles.is_empty() {
            spans.pop();
        } else {
            last.text = trimmed;
            break;
        }
    }
    expand_wiki_spans(coalesce_spans(spans))
}

/// Une tramos adyacentes con idénticos estilos y destino: `pulldown-cmark`
/// 0.12 emite `[` y `]` como eventos de texto separados (son potenciales
/// corchetes de link), así que `[[destino]]` nunca llega en un solo tramo.
/// La fusión es neutra para el render (mismos tags aplicados al texto
/// concatenado) y deja a `expand_wiki_spans` ver el `[[...]]` completo.
/// Pura — testeable sin display.
fn coalesce_spans(spans: Vec<RichSpan>) -> Vec<RichSpan> {
    let mut out: Vec<RichSpan> = Vec::with_capacity(spans.len());
    for span in spans {
        let mergeable = match out.last() {
            Some(last) => {
                last.styles == span.styles
                    && last.link_url == span.link_url
                    && last.wiki_target.is_none()
                    && span.wiki_target.is_none()
            }
            None => false,
        };
        if mergeable {
            out.last_mut().expect("coalesce: last checked").text.push_str(&span.text);
        } else {
            out.push(span);
        }
    }
    out
}

/// Parte los tramos que contienen `[[destino]]` en sub-tramos con estilo
/// `WikiLink` (mismo look que los markdown-links) y `wiki_target` con el
/// texto interior. `pulldown-cmark` no entiende la sintaxis wiki, así que
/// llega como texto literal: se detecta acá para que la preview la resalte
/// y la haga clickeable igual que el editor.
///
/// No toca tramos que ya son markdown-links (evita anidar destinos) ni
/// vacía el destino (`[[]]` queda como texto). Pura — testeable sin display.
fn expand_wiki_spans(spans: Vec<RichSpan>) -> Vec<RichSpan> {
    let mut out: Vec<RichSpan> = Vec::with_capacity(spans.len());
    for span in spans {
        if span.styles.contains(&MdStyle::Link) || !span.text.contains("[[") {
            out.push(span);
            continue;
        }
        let text = span.text.clone();
        let mut cursor = 0;
        let mut emitted = false;
        while let Some(rel_open) = text[cursor..].find("[[") {
            let open = cursor + rel_open;
            let after_open = open + 2;
            let Some(rel_close) = text[after_open..].find("]]") else {
                break;
            };
            let close = after_open + rel_close;
            let target = text[after_open..close].trim();
            if target.is_empty() || target.contains('\n') {
                cursor = after_open;
                continue;
            }
            if open > cursor {
                out.push(RichSpan {
                    text: text[cursor..open].to_string(),
                    styles: span.styles.clone(),
                    link_url: span.link_url.clone(),
                    wiki_target: None,
                });
            }
            let mut wiki_styles = span.styles.clone();
            if !wiki_styles.contains(&MdStyle::WikiLink) {
                wiki_styles.push(MdStyle::WikiLink);
            }
            out.push(RichSpan {
                text: text[open..close + 2].to_string(),
                styles: wiki_styles,
                link_url: None,
                wiki_target: Some(target.to_string()),
            });
            cursor = close + 2;
            emitted = true;
        }
        if !emitted {
            out.push(span);
        } else if cursor < text.len() {
            out.push(RichSpan {
                text: text[cursor..].to_string(),
                styles: span.styles.clone(),
                link_url: span.link_url.clone(),
                wiki_target: None,
            });
        }
    }
    out
}

/// Crea (o actualiza) los TextTags de preview del buffer. Idempotente:
/// cada llamada deja peso/escala/monospace/subrayado más los colores de la
/// paleta activa — headings y links en el acento `mauve` (mismo slot que
/// `wiki_link_accent`), código con fondo `surface0` (ver
/// `preview_code_background`). Sin hex hardcodeado: `accent` y `code_bg`
/// siempre vienen de la paleta vigente.
///
/// Llamar en cada render y en cada cambio de tema: actualizar la propiedad
/// del tag repinta los rangos ya aplicados sin tocar el texto.
fn ensure_preview_tags(buffer: &gtk4::TextBuffer, accent: &str, code_bg: &str) {
    use gtk4::pango;
    let tag = |name: &str| {
        if let Some(existing) = buffer.tag_table().lookup(name) {
            return existing;
        }
        let t = gtk4::TextTag::new(Some(name));
        buffer.tag_table().add(&t);
        t
    };
    let h1 = tag(md_tag_name(MdStyle::H1));
    h1.set_scale(1.5);
    h1.set_weight(700);
    h1.set_foreground(Some(accent));
    let h2 = tag(md_tag_name(MdStyle::H2));
    h2.set_scale(1.3);
    h2.set_weight(700);
    h2.set_foreground(Some(accent));
    let h3 = tag(md_tag_name(MdStyle::H3));
    h3.set_scale(1.15);
    h3.set_weight(700);
    h3.set_foreground(Some(accent));
    let bold = tag(md_tag_name(MdStyle::Bold));
    bold.set_weight(700);
    let italic = tag(md_tag_name(MdStyle::Italic));
    italic.set_style(pango::Style::Italic);
    let code = tag(md_tag_name(MdStyle::Code));
    code.set_family(Some("monospace"));
    code.set_background(Some(code_bg));
    let link = tag(md_tag_name(MdStyle::Link));
    link.set_underline(pango::Underline::Single);
    link.set_foreground(Some(accent));
    let wiki = tag(md_tag_name(MdStyle::WikiLink));
    wiki.set_underline(pango::Underline::Single);
    wiki.set_foreground(Some(accent));
}

/// Renderiza markdown al buffer de preview (reemplazo total) y devuelve los
/// rangos clickeables (offsets en caracteres). El buffer del editor no se
/// toca: el contenido y el cursor se preservan solos.
fn render_markdown_to_buffer(
    text: &str,
    buffer: &gtk4::TextBuffer,
    accent: &str,
    code_bg: &str,
) -> Vec<PreviewLink> {
    ensure_preview_tags(buffer, accent, code_bg);
    buffer.set_text("");
    let mut links = Vec::new();
    let mut offset: i32 = 0;
    for span in markdown_spans(text) {
        let names: Vec<&str> = span.styles.iter().map(|s| md_tag_name(*s)).collect();
        let mut end = buffer.end_iter();
        buffer.insert_with_tags_by_name(&mut end, &span.text, &names);
        let len = span.text.chars().count() as i32;
        if span.link_url.is_some() || span.wiki_target.is_some() {
            links.push(PreviewLink {
                start: offset,
                end: offset + len,
                url: span.link_url.clone(),
                wiki: span.wiki_target.clone(),
            });
        }
        offset += len;
    }
    links
}

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
    pub editor_stack: Stack,
    pub preview_view: TextView,
    pub toggle_preview: Rc<dyn Fn()>,
    pub sticky_title: Label,
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
    RandomNote,
    TogglePreview,
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
        key: Key::g,
        with_ctrl: true,
        section: "Generales",
        keys_label: "Ctrl+G",
        description: "Nota aleatoria",
        action: ShortcutAction::RandomNote,
    },
    WindowShortcut {
        key: Key::e,
        with_ctrl: true,
        section: "Generales",
        keys_label: "Ctrl+E",
        description: "Alternar vista previa",
        action: ShortcutAction::TogglePreview,
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

/// Lee la preferencia claro/oscuro del OS desde los GTK settings del display.
///
/// `true` = oscuro. Sin display o si la lectura falla, cae a oscuro (el look
/// Mocha es el arranque histórico); el CSS sigue al OS en vivo vía sus
/// bloques `@media`, solo el tag del wiki-link necesita un color concreto al
/// arrancar y en cada cambio de tema.
/// El cambio de variante del OS en caliente no re-colorea el tag (follow-up).
fn os_prefers_dark(display: Option<&gdk::Display>) -> bool {
    display
        .map(gtk4::Settings::for_display)
        .map(|settings| settings.is_gtk_application_prefer_dark_theme())
        .unwrap_or(true)
}

/// Solo `http://` y `https://` abren navegador desde la preview. Otros
/// esquemas (o `None`) se ignoran: nada que ejecutar, nada que romper.
/// Pura — testeable sin display.
fn is_http_url(url: Option<&str>) -> bool {
    matches!(url, Some(u) if u.starts_with("http://") || u.starts_with("https://"))
}

/// Abre una URL en el navegador predeterminado vía
/// `gio::AppInfo::launch_default_for_uri` (el estándar freedesktop, sin
/// ventana padre necesaria: la preview no tiene diálogo propio). Los
/// errores se ignoran a propósito: un click en un link roto no debe
/// voltear la app.
fn open_url_external(url: &str, parent: &TextView) {
    let context = parent.display().app_launch_context();
    let _ = gtk4::gio::AppInfo::launch_default_for_uri(url, Some(&context));
}

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

    // Vista previa de solo lectura: mismo padding que el editor para que el
    // texto no "salte" al alternar. No editable, cursor invisible.
    let preview_view = TextView::builder()
        .wrap_mode(gtk4::WrapMode::WordChar)
        .monospace(false)
        .editable(false)
        .cursor_visible(false)
        .left_margin(16)
        .right_margin(16)
        .top_margin(16)
        .bottom_margin(16)
        .build();

    let preview_scroll = ScrolledWindow::builder()
        .child(&preview_view)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vexpand(true)
        .build();

    // Stack edición/preview: vive como child del overlay (los overlays wiki
    // y de resultados van ENCIMA, así que no se rompen) y alterna qué hijo
    // se muestra. Nombres fijos para el test del toggle.
    let editor_stack = Stack::new();
    editor_stack.add_named(&text_scroll, Some("editor"));
    editor_stack.add_named(&preview_scroll, Some("preview"));
    editor_stack.set_visible_child_name("editor");
    let preview_shown: Rc<Cell<bool>> = Rc::new(Cell::new(false));

    // Sticky title header: pinned above the scrolled editor (outside
    // `text_scroll`, so it never scrolls away) showing the first non-blank
    // content line live. The body keeps line 1 intact — no buffer surgery —
    // so at the very top the title reads twice (header + first line) until
    // the user scrolls. Saved text stays byte-identical.
    let sticky_title = Label::new(Some("(nota vacía)"));
    sticky_title.set_halign(Align::Fill);
    sticky_title.set_xalign(0.0);
    sticky_title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    sticky_title.set_hexpand(true);
    sticky_title.add_css_class("heading");
    sticky_title.set_margin_start(16);
    sticky_title.set_margin_end(16);
    sticky_title.set_margin_top(8);
    sticky_title.set_margin_bottom(4);
    sticky_title.set_tooltip_text(Some("(nota vacía)"));
    editor_box.append(&sticky_title);

    let editor_overlay = Overlay::new();
    editor_overlay.set_child(Some(&editor_stack));
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

    // El resaltado [[...]] vive en WikiAutocomplete, que se crea más abajo:
    // este holder puentea el selector de tema con el setter de acento.
    let wiki_holder: Rc<RefCell<Option<WikiAutocomplete>>> = Rc::new(RefCell::new(None));
    // Lo mismo para la preview: el selector recalcula sus colores y
    // re-renderiza si está visible. Se rellena tras definir `render_preview`.
    let preview_theme_refresh: Rc<RefCell<Option<Rc<dyn Fn()>>>> =
        Rc::new(RefCell::new(None));

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
        let wiki_holder_c = wiki_holder.clone();
        let preview_theme_refresh_c = preview_theme_refresh.clone();
        item.connect_toggled(move |button| {
            if !button.is_active() {
                return;
            }
            if let Some(ref handle) = theme_handle_c {
                handle.apply(id);
            }
            // El CSS no toca el tag wiki-link: re-colorearlo al mauve de la
            // paleta nueva y re-resaltar, con la variante claro/oscuro del OS.
            let dark = os_prefers_dark(gdk::Display::default().as_ref());
            if let Some(ref wiki) = *wiki_holder_c.borrow() {
                wiki.set_accent(&wiki_link_accent(id, dark));
            }
            state_c.borrow_mut().config.theme = id.as_str().to_string();
            state_c.borrow().config.save();
            // La preview resuelve sus colores desde el estado ya actualizado
            // y re-renderiza solo si está visible (ver `render_preview`).
            if let Some(ref refresh) = *preview_theme_refresh_c.borrow() {
                refresh();
            }
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
    // overlay de resultados del modo vertical. Los tags son clickeables:
    // filtran la lista vía `search_entry` (el `connect_search_changed`
    // existente re-filtra y repuebla, sin tocar `filtered_indices` a mano).
    let build_note_row = {
        let search_entry = search_entry.clone();
        Rc::new(move |note: &nv_core::note::Note| -> ListBoxRow {
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

        // Meta de la fila (parity slice 1): SOLO fecha/hora de modificación
        // (formato canónico `formatted_date`) + un widget por tag. Cada tag
        // es clickeable y filtra la lista vía `search_entry` (toggle: si la
        // query ya es `#tag`, la limpia). El re-filtrado lo hace la señal
        // `connect_search_changed` existente. Sin fecha de creación.
        let meta_box = GtkBox::new(Orientation::Horizontal, 6);
        meta_box.set_hexpand(true);
        // Blanco de click generoso: la fila meta es un poco más alta para
        // que los tags-botón sean fáciles de acertar con el mouse.
        meta_box.set_margin_top(3);
        meta_box.set_margin_bottom(3);
        let date_label = Label::new(Some(&note.formatted_date()));
        date_label.set_halign(Align::Start);
        date_label.set_xalign(0.0);
        date_label.add_css_class("caption");
        date_label.add_css_class("dim-label");
        date_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        date_label.set_tooltip_text(Some(&note.formatted_date()));
        meta_box.append(&date_label);
        for tag in &note.tags {
            // Button plano en vez de Label+GestureClick: el ListBoxRow
            // reclama el click en fase bubble para selección/activación y
            // el handler del tag nunca corría. Los botones sí reciben el
            // click dentro de la row, así que el filtro por tag funciona.
            let tag_button = Button::with_label(&format!("#{tag}"));
            tag_button.add_css_class("flat");
            tag_button.add_css_class("caption");
            tag_button.add_css_class("nv-tag");
            tag_button.set_focusable(false);
            tag_button.set_tooltip_text(Some(&format!("Filtrar por #{tag}")));
            {
                let search_entry = search_entry.clone();
                let wanted = format!("#{tag}");
                tag_button.connect_clicked(move |_| {
                    if search_entry.text().as_str() == wanted {
                        search_entry.set_text("");
                    } else {
                        search_entry.set_text(&wanted);
                    }
                });
            }
            meta_box.append(&tag_button);
        }

        row_box.append(&title_label);
        row_box.append(&meta_box);

        let row = ListBoxRow::new();
        row.set_child(Some(&row_box));
        row
        })
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

    // Sistema de wiki-links: resaltado + autocompletado difuso con panel flotante.
    // El tag arranca con el mauve de la paleta inicial (variante según el OS).
    let initial_dark = os_prefers_dark(gdk::Display::default().as_ref());
    let initial_accent = wiki_link_accent(initial_theme, initial_dark);
    let wiki = WikiAutocomplete::setup(
        Rc::clone(&state),
        text_view.clone(),
        editor_overlay,
        &initial_accent,
        {
            let search_wiki_target = search_wiki_target.clone();
            move |target: &str| search_wiki_target(target)
        },
    );
    *wiki_holder.borrow_mut() = Some(wiki.clone());

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

    // Rangos clickeables de la última renderización de preview
    // (URLs http(s) + [[wiki-links]]). Los actualiza `render_preview` y los
    // leen el hover/click de `preview_view`. Solo vive en la preview.
    let preview_links: Rc<RefCell<Vec<PreviewLink>>> = Rc::new(RefCell::new(Vec::new()));

    // Renderiza el texto ACTUAL del buffer del editor en la vista previa.
    // No toca el buffer del editor: contenido y cursor se preservan solos.
    // Los colores se resuelven acá desde la paleta activa (mauve para
    // headings/links, surface0 de fondo para código), así que un cambio de
    // tema queda aplicado con solo re-renderizar.
    let render_preview = {
        let text_view = text_view.clone();
        let preview_view = preview_view.clone();
        let preview_links = Rc::clone(&preview_links);
        let state = Rc::clone(&state);

        move || {
            let buffer = text_view.buffer();
            let (start, end) = buffer.bounds();
            let text = buffer.text(&start, &end, true).to_string();
            drop(buffer);
            let theme = ThemeId::from_persisted(&state.borrow().config.theme);
            let dark = os_prefers_dark(gdk::Display::default().as_ref());
            let accent = wiki_link_accent(theme, dark);
            let code_bg = preview_code_background(theme, dark);
            let links =
                render_markdown_to_buffer(&text, &preview_view.buffer(), &accent, &code_bg);
            *preview_links.borrow_mut() = links;
        }
    };

    // Aplica la paleta vigente a los tags ya creados y re-renderiza si la
    // preview está visible. Lo invoca el selector de tema (vía holder,
    // porque se define después del loop del picker).
    *preview_theme_refresh.borrow_mut() = Some(Rc::new({
        let preview_view = preview_view.clone();
        let preview_shown = Rc::clone(&preview_shown);
        let render_preview = render_preview.clone();
        let state = Rc::clone(&state);
        move || {
            let theme = ThemeId::from_persisted(&state.borrow().config.theme);
            let dark = os_prefers_dark(gdk::Display::default().as_ref());
            ensure_preview_tags(
                &preview_view.buffer(),
                &wiki_link_accent(theme, dark),
                &preview_code_background(theme, dark),
            );
            if preview_shown.get() {
                render_preview();
            }
        }
    }));

    // Links clickeables solo en la preview (el editor no se toca):
    // - http(s): manito + tooltip con la URL + click abre el navegador.
    // - [[wiki]]: mismo gesto que en el editor (filtra por el destino vía
    //   `search_wiki_target`, igual que Ctrl+Enter sobre el editor).
    {
        let motion_view = preview_view.clone();
        let motion_links = Rc::clone(&preview_links);
        let motion = gtk4::EventControllerMotion::new();
        motion.connect_motion(move |_, x, y| {
            let found = motion_view
                .iter_at_position(x as i32, y as i32)
                .and_then(|(iter, _)| {
                    let offset = iter.offset();
                    motion_links
                        .borrow()
                        .iter()
                        .find(|l| offset >= l.start && offset < l.end)
                        .cloned()
                });
            match found {
                Some(link) if is_http_url(link.url.as_deref()) => {
                    let cursor = gdk::Cursor::from_name("pointer", None);
                    motion_view.set_cursor(cursor.as_ref());
                    motion_view.set_tooltip_text(link.url.as_deref());
                }
                Some(link) if link.wiki.is_some() => {
                    let cursor = gdk::Cursor::from_name("pointer", None);
                    motion_view.set_cursor(cursor.as_ref());
                    motion_view.set_tooltip_text(link.wiki.as_deref());
                }
                _ => {
                    motion_view.set_cursor(None::<&gdk::Cursor>);
                    motion_view.set_tooltip_text(None);
                }
            }
        });
        motion.connect_leave({
            let motion_view = preview_view.clone();
            move |_| {
                motion_view.set_cursor(None::<&gdk::Cursor>);
                motion_view.set_tooltip_text(None);
            }
        });
        preview_view.add_controller(motion);

        let click_view = preview_view.clone();
        let click_links = Rc::clone(&preview_links);
        let search_wiki_target_c = search_wiki_target.clone();
        let click = gtk4::GestureClick::new();
        click.set_button(gdk::BUTTON_PRIMARY);
        click.connect_pressed(move |_, n_press, x, y| {
            if n_press != 1 {
                return;
            }
            let Some((iter, _)) = click_view.iter_at_position(x as i32, y as i32) else {
                return;
            };
            let offset = iter.offset();
            let found = click_links
                .borrow()
                .iter()
                .find(|l| offset >= l.start && offset < l.end)
                .cloned();
            match found {
                Some(link) if is_http_url(link.url.as_deref()) => {
                    open_url_external(link.url.as_deref().unwrap_or_default(), &click_view);
                }
                Some(link) => {
                    if let Some(target) = link.wiki.as_deref() {
                        search_wiki_target_c(target);
                    }
                }
                None => {}
            }
        });
        preview_view.add_controller(click);
    }

    // Toggle edición/preview (Ctrl+E). Entrar renderiza el texto actual;
    // salir vuelve al editor con foco. El sticky title no se toca: sigue
    // mostrando el título vivo como siempre.
    let toggle_preview: Rc<dyn Fn()> = Rc::new({
        let editor_stack = editor_stack.clone();
        let text_view = text_view.clone();
        let preview_view = preview_view.clone();
        let preview_shown = Rc::clone(&preview_shown);
        let render_preview = render_preview.clone();

        move || {
            if preview_shown.get() {
                editor_stack.set_visible_child_name("editor");
                preview_shown.set(false);
                text_view.grab_focus();
            } else {
                render_preview();
                editor_stack.set_visible_child_name("preview");
                preview_shown.set(true);
                preview_view.grab_focus();
            }
        }
    });

    let select_note_by_id = {
        let state = Rc::clone(&state);
        let text_view = text_view.clone();
        let active_list_box = active_list_box.clone();
        let info_label = info_label.clone();
        let sticky_title = sticky_title.clone();
        let flush_pending_save = flush_pending_save.clone();
        let preview_shown = Rc::clone(&preview_shown);
        let render_preview = render_preview.clone();
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

                // En preview, la nota nueva se muestra renderizada (el modo
                // se mantiene al navegar, como el sticky que sigue vivo).
                if preview_shown.get() {
                    render_preview();
                }

                let title = sticky_title_for(&content);
                sticky_title.set_text(&title);
                sticky_title.set_tooltip_text(Some(&title));

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
        let sticky_title = sticky_title.clone();
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

            let title = sticky_title_for(&text);
            sticky_title.set_text(&title);
            sticky_title.set_tooltip_text(Some(&title));

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

    // Keyboard Controller for Global App Shortcuts (Ctrl+L, Esc, Ctrl+N, Ctrl+D, Ctrl+T, Ctrl+R, Ctrl+J, Ctrl+K, Ctrl+P, Ctrl+G)
    let key_controller = EventControllerKey::new();
    key_controller.connect_key_pressed({
        let state = Rc::clone(&state);
        let select_note_by_id = select_note_by_id.clone();
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
        let toggle_preview = Rc::clone(&toggle_preview);

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
                ShortcutAction::TogglePreview => toggle_preview(),
                ShortcutAction::RandomNote => {
                    // Nota aleatoria: de la lista filtrada si hay filtro
                    // activo, si no de todas las notas. Lista vacía = no-op.
                    let target_id = {
                        let st = state.borrow();
                        let pool: Vec<String> = if st.filtered_indices.is_empty() {
                            st.storage.notes.iter().map(|n| n.id.clone()).collect()
                        } else {
                            st.filtered_indices.clone()
                        };
                        if pool.is_empty() {
                            None
                        } else {
                            let nanos = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map(|d| d.subsec_nanos() as usize)
                                .unwrap_or(0);
                            pool.get(nanos % pool.len()).cloned()
                        }
                    };
                    if let Some(id) = target_id {
                        select_note_by_id(&id);
                    }
                }
                ShortcutAction::EscapeContextual => {
                    // Con query activa, Esc la limpia (la señal
                    // `search_changed` repuebla la lista); si no,
                    // comportamiento previo.
                    if !search_entry.text().trim().is_empty() {
                        search_entry.set_text("");
                    } else if is_portrait.get() && results_panel.is_visible() {
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
        editor_stack: editor_stack.clone(),
        preview_view: preview_view.clone(),
        toggle_preview: Rc::clone(&toggle_preview),
        sticky_title: sticky_title.clone(),
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
    fn sticky_title_for_skips_blanks_and_falls_back() {
        // Empty note: shared fallback, never empty, never crashes.
        assert_eq!(sticky_title_for(""), "(nota vacía)");
        assert_eq!(sticky_title_for("   \n\t\n  "), "(nota vacía)");
        // Leading blank lines are skipped; first live line wins verbatim.
        assert_eq!(sticky_title_for("\n  \nHola\nmundo"), "Hola");
        assert_eq!(sticky_title_for("  # Título\ncuerpo"), "# Título");
        // Unicode titles survive intact (visual ellipsis only, never in data).
        assert_eq!(sticky_title_for("\n\n日本語タイトル\ncuerpo"), "日本語タイトル");
        assert_eq!(sticky_title_for("café ☕ mañana"), "café ☕ mañana");
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
            ShortcutAction::RandomNote,
            ShortcutAction::TogglePreview,
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

    #[test]
    fn markdown_spans_renders_basic_subset() {
        let spans = markdown_spans(
            "# Título\n\nHola **negrita** y *itálica* con `código`.\n\n- uno\n- dos\n\n[enlace](https://ejemplo.com)\n",
        );
        let has = |needle: &str, style: MdStyle| {
            spans
                .iter()
                .any(|s| s.text.contains(needle) && s.styles.contains(&style))
        };
        assert!(has("Título", MdStyle::H1), "heading con H1: {spans:?}");
        assert!(has("negrita", MdStyle::Bold));
        assert!(has("itálica", MdStyle::Italic));
        assert!(has("código", MdStyle::Code));
        // Bullets de lista presentes como texto.
        assert!(
            spans.iter().any(|s| s.text.contains('•')),
            "bullets: {spans:?}"
        );
        // Links como texto: el texto + la URL visible entre paréntesis.
        assert!(has("enlace", MdStyle::Link));
        assert!(
            spans
                .iter()
                .any(|s| s.text.contains("https://ejemplo.com")),
            "url visible: {spans:?}"
        );
        // Texto normal sin estilos arrastrados.
        let hola = spans.iter().find(|s| s.text.contains("Hola ")).unwrap();
        assert!(hola.styles.is_empty(), "texto normal limpio: {hola:?}");
    }

    #[test]
    fn markdown_spans_code_block_ordered_list_and_rule() {
        let spans = markdown_spans("1. primero\n2. segundo\n\n```\nlet x = 1;\n```\n\n---\n");
        assert!(
            spans.iter().any(|s| s.text.contains("1. ")),
            "ordenada numera: {spans:?}"
        );
        assert!(
            spans
                .iter()
                .any(|s| s.text.contains("let x = 1;") && s.styles.contains(&MdStyle::Code)),
            "bloque de código: {spans:?}"
        );
        assert!(
            spans.iter().any(|s| s.text.contains("───")),
            "regla: {spans:?}"
        );
        // Sin newlines colgantes al final.
        let last = spans.last().unwrap();
        assert!(
            !last.text.ends_with('\n'),
            "sin trailing newline: {last:?}"
        );
    }

    #[test]
    fn markdown_spans_heading_levels() {
        let spans = markdown_spans("# uno\n\n## dos\n\n### tres\n");
        let style_of = |needle: &str| {
            spans
                .iter()
                .find(|s| s.text.contains(needle))
                .map(|s| s.styles.clone())
                .unwrap_or_default()
        };
        assert!(style_of("uno").contains(&MdStyle::H1));
        assert!(style_of("dos").contains(&MdStyle::H2));
        assert!(style_of("tres").contains(&MdStyle::H3));
    }

    #[test]
    fn markdown_spans_propagates_link_url_to_text_and_suffix() {
        let spans = markdown_spans("[enlace](https://ejemplo.com)\n");
        let linked: Vec<_> = spans.iter().filter(|s| s.link_url.is_some()).collect();
        assert!(
            !linked.is_empty(),
            "tramos con destino: {spans:?}"
        );
        for s in &linked {
            assert_eq!(s.link_url.as_deref(), Some("https://ejemplo.com"));
            assert!(s.styles.contains(&MdStyle::Link));
        }
        // Etiqueta y URL visible quedan cubiertas por el destino (pueden
        // llegar fusionadas en un solo tramo: mismos tags, mismo link).
        assert!(
            linked.iter().any(|s| s.text.contains("enlace")),
            "etiqueta: {spans:?}"
        );
        assert!(
            spans
                .iter()
                .any(|s| s.text.contains("https://ejemplo.com")),
            "url visible: {spans:?}"
        );
    }

    #[test]
    fn markdown_spans_splits_wiki_links_with_target() {
        let spans = markdown_spans("ver [[mi nota]] y seguir\n");
        let wiki = spans
            .iter()
            .find(|s| s.text == "[[mi nota]]")
            .unwrap_or_else(|| panic!("wiki-link partido: {spans:?}"));
        assert!(wiki.styles.contains(&MdStyle::WikiLink));
        assert_eq!(wiki.wiki_target.as_deref(), Some("mi nota"));
        // El texto alrededor se conserva como tramos propios sin destino.
        assert!(spans.iter().any(|s| s.text.contains("ver ")));
        assert!(spans.iter().any(|s| s.text.contains(" y seguir")));
    }

    #[test]
    fn markdown_spans_ignores_unclosed_or_empty_wiki() {
        for raw in ["[[sin cerrar\n", "[[]]\n", "[[  ]]\n"] {
            let spans = markdown_spans(raw);
            assert!(
                spans.iter().all(|s| s.wiki_target.is_none()),
                "sin destino wiki en {raw:?}: {spans:?}"
            );
        }
    }

    #[test]
    fn preview_code_background_matches_surface0_slot() {
        for theme in [
            ThemeId::Catppuccin,
            ThemeId::Dracula,
            ThemeId::Flexoki,
            ThemeId::Wallpaper,
        ] {
            for dark in [false, true] {
                let variant = if dark { Variant::Dark } else { Variant::Light };
                assert_eq!(
                    preview_code_background(theme, dark),
                    palette(theme, variant).surface0.into_owned(),
                    "{theme:?} dark={dark}",
                );
            }
        }
    }

    #[test]
    fn only_http_urls_open_externally() {
        assert!(is_http_url(Some("http://ejemplo.com")));
        assert!(is_http_url(Some("https://ejemplo.com/x")));
        for bad in [
            None,
            Some(""),
            Some("ftp://ejemplo.com"),
            Some("file:///tmp/x"),
            Some("javascript:alert(1)"),
            Some("ejemplo.com"),
        ] {
            assert!(!is_http_url(bad), "no debe abrir: {bad:?}");
        }
    }

    // Requiere display: `xvfb-run -a cargo test -- --test-threads=1`
    // Test único con display (GTK solo permite init desde un hilo y libtest
    // usa un hilo por test): cubre layout responsivo + toggle de preview.
    // La orientación la dispara el tick callback (probado por smoke: al
    // redimensionar 500x800 el log muestra `apply_layout -> portrait=true`);
    // aquí se usa el seam expuesto para probar el layout de forma determinista.
    #[test]
    fn responsive_layout_overlay_and_preview_toggle() {
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

        // Preview (Ctrl+E lógico): arranca en edición, el toggle renderiza
        // el texto actual y muestra la preview; el segundo toggle vuelve al
        // editor con el contenido intacto.
        assert_eq!(
            handles.editor_stack.visible_child_name().as_deref(),
            Some("editor")
        );
        handles.text_view.buffer().set_text("# Hola\n\nTexto **fuerte**.");
        pump(20);
        (handles.toggle_preview)();
        pump(20);
        assert_eq!(
            handles.editor_stack.visible_child_name().as_deref(),
            Some("preview")
        );
        {
            let buffer = handles.preview_view.buffer();
            let (start, end) = buffer.bounds();
            let rendered = buffer.text(&start, &end, true).to_string();
            assert!(
                rendered.contains("Hola") && rendered.contains("fuerte"),
                "preview con contenido: {rendered:?}"
            );
            assert!(
                !rendered.contains("**") && !rendered.contains('#'),
                "renderizado, no crudo: {rendered:?}"
            );
        }
        assert!(!handles.preview_view.is_editable());
        (handles.toggle_preview)();
        pump(20);
        assert_eq!(
            handles.editor_stack.visible_child_name().as_deref(),
            Some("editor")
        );
        {
            let buffer = handles.text_view.buffer();
            let (start, end) = buffer.bounds();
            let back = buffer.text(&start, &end, true).to_string();
            assert!(
                back.contains("# Hola"),
                "el editor conserva el crudo: {back:?}"
            );
        }
    }
}
