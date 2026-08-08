use chrono::Local;
use gtk4::glib;

use crate::config::Config;
use crate::storage::StorageManager;
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

/// Genera un título único basado en la fecha/hora actual: YYYYMMDD-HHMM
pub fn timestamp_title() -> String {
    Local::now().format("%Y%m%d-%H%M").to_string()
}
