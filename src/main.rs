mod config;
mod note;
mod search;
mod storage;
mod wiki_link;
mod window;

use libadwaita::prelude::*;
use libadwaita::Application;

fn main() {
    let app = Application::builder()
        .application_id("org.notational.velocity")
        .build();

    app.connect_activate(window::build_ui);

    app.run();
}
