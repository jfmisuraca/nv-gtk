use chrono::Local;

/// Genera un título único basado en la fecha/hora actual: YYYYMMDD-HHMM
pub fn timestamp_title() -> String {
    Local::now().format("%Y%m%d-%H%M").to_string()
}

/// Versión con segundos (YYYYMMDD-HHMMSS): desambiguar notas creadas dentro
/// del mismo minuto cuando el título base ya está ocupado.
pub fn timestamp_title_with_seconds() -> String {
    Local::now().format("%Y%m%d-%H%M%S").to_string()
}
