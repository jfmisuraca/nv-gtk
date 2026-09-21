//! Catppuccin theme for the desktop app.
//!
//! Installs a single application-wide CSS provider that overrides the
//! libadwaita named colors: Latte for the light variant, Mocha for the
//! dark variant. The active variant follows the OS color scheme with no
//! manual toggle:
//!
//! - The stylesheet uses `@media (prefers-color-scheme: …)` blocks.
//! - Since GTK 4.20 that media feature is evaluated against the provider's
//!   own `prefers-color-scheme` property, so `load` binds it to the display
//!   settings' `gtk-interface-color-scheme` (the system-wide preference) —
//!   exactly what GTK does for its built-in theme provider, and what keeps
//!   runtime OS switches working.

use gtk4::gdk;
use gtk4::prelude::*;

/// Catppuccin mappings for the libadwaita color variables, following the
/// official Catppuccin GTK port (https://catppuccin.com/ports).
///
/// Note: the spec originally asked for `@variant (light)` blocks, but the
/// GTK CSS parser (verified against 4.22.5) rejects that at-rule and drops
/// the block ("Unknown @ rule"). `@media (prefers-color-scheme: ...)` is
/// GTK's supported conditional (the same one libadwaita itself uses) and
/// matches the active light/dark variant, so the OS-switching behavior is
/// identical.
const CATPPUCCIN_CSS: &str = r#"
/* NV-GTK Catppuccin theme
 * Light variant -> Latte, dark variant -> Mocha.
 * The active variant follows the OS color scheme automatically.
 */

@media (prefers-color-scheme: light) {
    /* ---------- Palette (Latte) ---------- */
    @define-color base       #eff1f5;
    @define-color mantle     #e6e9ef;
    @define-color crust      #dce0e8;
    @define-color surface0   #ccd0da;
    @define-color surface1   #bcc0cc;
    @define-color surface2   #acb0be;
    @define-color overlay0   #9ca0b0;
    @define-color overlay1   #8c8fa1;
    @define-color overlay2   #7c7f93;
    @define-color subtext0   #6c6f85;
    @define-color subtext1   #5c5f77;
    @define-color text       #4c4f69;
    @define-color lavender   #7287fd;
    @define-color blue       #1e66f5;
    @define-color mauve      #8839ef;
    @define-color red        #d20f39;
    @define-color green      #40a02b;
    @define-color yellow     #df8e1d;
    @define-color teal       #179299;
    @define-color sky        #04a5e5;
    @define-color pink       #ea76cb;
    @define-color peach      #fe640b;

    /* ---------- Window & views ---------- */
    @define-color window_bg_color @base;
    @define-color window_fg_color @text;
    @define-color view_bg_color @base;
    @define-color view_fg_color @text;
    @define-color text_view_bg_color @view_bg_color;
    @define-color text_view_fg_color @view_fg_color;

    /* ---------- Header bar & sidebar ---------- */
    @define-color headerbar_bg_color @mantle;
    @define-color headerbar_fg_color @text;
    @define-color headerbar_shade_color alpha(@surface1, 0.36);
    @define-color headerbar_border_color @surface1;
    @define-color sidebar_bg_color @mantle;
    @define-color sidebar_fg_color @text;
    @define-color sidebar_border_color @surface1;

    /* ---------- Cards, popovers & OSD ---------- */
    @define-color card_bg_color @surface0;
    @define-color card_fg_color @text;
    @define-color card_shade_color @base;
    @define-color popover_bg_color @surface0;
    @define-color popover_fg_color @text;
    @define-color osd_bg_color @surface0;
    @define-color osd_fg_color @text;

    /* ---------- Borders ---------- */
    @define-color borders @surface1;

    /* ---------- Accent & status ---------- */
    @define-color accent_bg_color @mauve;
    @define-color accent_fg_color @base;
    @define-color accent_color @mauve;
    @define-color destructive_bg_color @red;
    @define-color destructive_fg_color @base;
    @define-color error_color @red;
    @define-color success_bg_color @green;
    @define-color success_fg_color @base;
    @define-color warning_bg_color @yellow;
    @define-color warning_fg_color @base;

    /* ---------- Disabled state ---------- */
    @define-color insensitive_fg_color @overlay1;
    @define-color insensitive_bg_color @mantle;

    /* ---------- Legacy GTK aliases (used by custom CSS) ---------- */
    @define-color theme_base_color @base;
    @define-color theme_bg_color @base;
    @define-color theme_fg_color @text;
    @define-color theme_text_color @text;
    @define-color theme_selected_bg_color @accent_bg_color;
    @define-color theme_selected_fg_color @accent_fg_color;
}

@media (prefers-color-scheme: dark) {
    /* ---------- Palette (Mocha) ---------- */
    @define-color base       #1e1e2e;
    @define-color mantle     #181825;
    @define-color crust      #11111b;
    @define-color surface0   #313244;
    @define-color surface1   #45475a;
    @define-color surface2   #585b70;
    @define-color overlay0   #6c7086;
    @define-color overlay1   #7f849c;
    @define-color overlay2   #9399b2;
    @define-color subtext0   #a6adc8;
    @define-color subtext1   #bac2de;
    @define-color text       #cdd6f4;
    @define-color lavender   #b4befe;
    @define-color blue       #89b4fa;
    @define-color mauve      #cba6f7;
    @define-color red        #f38ba8;
    @define-color green      #a6e3a1;
    @define-color yellow     #f9e2af;
    @define-color teal       #94e2d5;
    @define-color sky        #89dceb;
    @define-color pink       #f5c2e7;
    @define-color peach      #fab387;

    /* ---------- Window & views ---------- */
    @define-color window_bg_color @base;
    @define-color window_fg_color @text;
    @define-color view_bg_color @base;
    @define-color view_fg_color @text;
    @define-color text_view_bg_color @view_bg_color;
    @define-color text_view_fg_color @view_fg_color;

    /* ---------- Header bar & sidebar ---------- */
    @define-color headerbar_bg_color @mantle;
    @define-color headerbar_fg_color @text;
    @define-color headerbar_shade_color alpha(@surface1, 0.36);
    @define-color headerbar_border_color @surface1;
    @define-color sidebar_bg_color @mantle;
    @define-color sidebar_fg_color @text;
    @define-color sidebar_border_color @surface1;

    /* ---------- Cards, popovers & OSD ---------- */
    @define-color card_bg_color @surface0;
    @define-color card_fg_color @text;
    @define-color card_shade_color @base;
    @define-color popover_bg_color @surface0;
    @define-color popover_fg_color @text;
    @define-color osd_bg_color @surface0;
    @define-color osd_fg_color @text;

    /* ---------- Borders ---------- */
    @define-color borders @surface1;

    /* ---------- Accent & status ---------- */
    @define-color accent_bg_color @mauve;
    @define-color accent_fg_color @base;
    @define-color accent_color @mauve;
    @define-color destructive_bg_color @red;
    @define-color destructive_fg_color @base;
    @define-color error_color @red;
    @define-color success_bg_color @green;
    @define-color success_fg_color @base;
    @define-color warning_bg_color @yellow;
    @define-color warning_fg_color @base;

    /* ---------- Disabled state ---------- */
    @define-color insensitive_fg_color @overlay1;
    @define-color insensitive_bg_color @mantle;

    /* ---------- Legacy GTK aliases (used by custom CSS) ---------- */
    @define-color theme_base_color @base;
    @define-color theme_bg_color @base;
    @define-color theme_fg_color @text;
    @define-color theme_text_color @text;
    @define-color theme_selected_bg_color @accent_bg_color;
    @define-color theme_selected_fg_color @accent_fg_color;
}
"#;

/// Loads the Catppuccin stylesheet for `display` at application priority.
///
/// The provider stays registered (and alive) with the display, matching how
/// the autocomplete panel installs its provider in `wiki_autocomplete.rs`.
pub fn load(display: &gdk::Display) {
    let provider = gtk4::CssProvider::new();
    provider.load_from_string(CATPPUCCIN_CSS);

    // GTK >= 4.20 evaluates `@media (prefers-color-scheme: …)` against the
    // provider's own `prefers-color-scheme` property, which defaults to
    // "light". Bind it to the display settings' `gtk-interface-color-scheme`
    // (the system-wide preference) so the blocks above follow the OS color
    // scheme — the same binding GTK applies to its built-in theme provider.
    // This also keeps runtime OS switches working. On GTK < 4.20 the media
    // feature was derived from the settings directly and needs no binding.
    let settings = gtk4::Settings::for_display(display);
    if gtk4::minor_version() >= 20 {
        let binding = settings
            .bind_property(
                "gtk-interface-color-scheme",
                &provider,
                "prefers-color-scheme",
            )
            .sync_create()
            .build();
        // The binding must outlive `load()` or the property stops
        // tracking scheme changes. Leak it deliberately: it is meant to
        // live for the process, like the provider registration.
        std::mem::forget(binding);
    }

    gtk4::style_context_add_provider_for_display(
        display,
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
