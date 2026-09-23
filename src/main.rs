mod app_state;
mod palettes;
mod pywal;
mod theme;
mod wiki_autocomplete;
mod wiki_link;
mod window;

use libadwaita::prelude::*;
use libadwaita::Application;

fn main() {
    let app = Application::builder()
        .application_id("org.notational.velocity")
        .build();

    app.connect_activate(|app| {
        let _handles = window::build_ui(app);
    });

    app.run();
}
