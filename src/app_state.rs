use gtk4::glib;

use nv_core::config::Config;
use nv_core::storage::StorageManager;
use crate::wiki_link::WikiLink;

/// Estado central de la aplicación: configuración, notas cargadas, filtro de
/// búsqueda activo, nota actualmente abierta, y lo necesario para el autoguardado
/// y el resaltado de wiki-links.
pub struct AppState {
    pub config: Config,
    pub storage: StorageManager,
    pub filtered_indices: Vec<String>,
    pub current_note_id: Option<String>,
    pub save_timeout_source: Option<glib::SourceId>,
    pub is_updating_ui: bool,
    pub current_wiki_links: Vec<WikiLink>,
}
