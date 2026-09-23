//! Palette-driven theme for the desktop app.
//!
//! Installs a single application-wide CSS provider whose libadwaita named
//! colors are generated from the selected [`palettes::ThemeId`] palette: the
//! `@media (prefers-color-scheme: light)` block carries the palette's light
//! variant and the `@media (prefers-color-scheme: dark)` block its dark
//! variant. The active variant follows the OS color scheme with no manual
//! toggle:
//!
//! - The stylesheet uses `@media (prefers-color-scheme: …)` blocks.
//! - Since GTK 4.20 that media feature is evaluated against the provider's
//!   own `prefers-color-scheme` property, so [`load`] binds it to the display
//!   settings' `gtk-interface-color-scheme` (the system-wide preference) —
//!   exactly what GTK does for its built-in theme provider, and what keeps
//!   runtime OS switches working.
//! - [`load`] retains the provider in a [`ThemeHandle`] so
//!   [`ThemeHandle::apply`] can re-target the palette at runtime without a
//!   restart.
//!
//! Note: the spec originally asked for `@variant (light)` blocks, but the
//! GTK CSS parser (verified against 4.22.5) rejects that at-rule and drops
//! the block ("Unknown @ rule"). `@media (prefers-color-scheme: ...)` is
//! GTK's supported conditional (the same one libadwaita itself uses) and
//! matches the active light/dark variant, so the OS-switching behavior is
//! identical.

use crate::palettes::{self, Palette, ThemeId, Variant};
use gtk4::gdk;
use gtk4::prelude::*;

/// Stylesheet template for the light variant. `%%NAME%%` tokens are replaced
/// with the palette's hex values by [`render_block`]; every other line is
/// literal CSS.
///
/// The literal `@media` / `@define-color` lines are load-bearing: the alias
/// parity check in `scripts/verify-theme.sh` greps this file between the two
/// `@media` markers, so the aliases stay spelled out instead of being built
/// programmatically.
const LIGHT_BLOCK: &str = r#"
/* NV-GTK desktop theme.
 * The palette is generated from the selected ThemeId; this block carries its
 * light variant. The active variant follows the OS color scheme automatically.
 */

@media (prefers-color-scheme: light) {
    /* ---------- Palette ---------- */
    @define-color base       %%BASE%%;
    @define-color mantle     %%MANTLE%%;
    @define-color crust      %%CRUST%%;
    @define-color surface0   %%SURFACE0%%;
    @define-color surface1   %%SURFACE1%%;
    @define-color surface2   %%SURFACE2%%;
    @define-color overlay0   %%OVERLAY0%%;
    @define-color overlay1   %%OVERLAY1%%;
    @define-color overlay2   %%OVERLAY2%%;
    @define-color subtext0   %%SUBTEXT0%%;
    @define-color subtext1   %%SUBTEXT1%%;
    @define-color text       %%TEXT%%;
    @define-color lavender   %%LAVENDER%%;
    @define-color blue       %%BLUE%%;
    @define-color mauve      %%MAUVE%%;
    @define-color red        %%RED%%;
    @define-color green      %%GREEN%%;
    @define-color yellow     %%YELLOW%%;
    @define-color teal       %%TEAL%%;
    @define-color sky        %%SKY%%;
    @define-color pink       %%PINK%%;
    @define-color peach      %%PEACH%%;

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

/// Stylesheet template for the dark variant. Structurally identical to
/// [`LIGHT_BLOCK`] — the named-color set must match in both variants — with
/// the same `%%NAME%%` tokens resolved against the dark palette.
const DARK_BLOCK: &str = r#"
/* NV-GTK desktop theme.
 * The palette is generated from the selected ThemeId; this block carries its
 * dark variant. The active variant follows the OS color scheme automatically.
 */

@media (prefers-color-scheme: dark) {
    /* ---------- Palette ---------- */
    @define-color base       %%BASE%%;
    @define-color mantle     %%MANTLE%%;
    @define-color crust      %%CRUST%%;
    @define-color surface0   %%SURFACE0%%;
    @define-color surface1   %%SURFACE1%%;
    @define-color surface2   %%SURFACE2%%;
    @define-color overlay0   %%OVERLAY0%%;
    @define-color overlay1   %%OVERLAY1%%;
    @define-color overlay2   %%OVERLAY2%%;
    @define-color subtext0   %%SUBTEXT0%%;
    @define-color subtext1   %%SUBTEXT1%%;
    @define-color text       %%TEXT%%;
    @define-color lavender   %%LAVENDER%%;
    @define-color blue       %%BLUE%%;
    @define-color mauve      %%MAUVE%%;
    @define-color red        %%RED%%;
    @define-color green      %%GREEN%%;
    @define-color yellow     %%YELLOW%%;
    @define-color teal       %%TEAL%%;
    @define-color sky        %%SKY%%;
    @define-color pink       %%PINK%%;
    @define-color peach      %%PEACH%%;

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

/// Replaces the `%%NAME%%` tokens in `template` with `palette`'s hex values.
fn render_block(template: &str, palette: &Palette) -> String {
    let mut css = template.to_string();
    for (token, value) in [
        ("%%BASE%%", palette.base),
        ("%%MANTLE%%", palette.mantle),
        ("%%CRUST%%", palette.crust),
        ("%%SURFACE0%%", palette.surface0),
        ("%%SURFACE1%%", palette.surface1),
        ("%%SURFACE2%%", palette.surface2),
        ("%%OVERLAY0%%", palette.overlay0),
        ("%%OVERLAY1%%", palette.overlay1),
        ("%%OVERLAY2%%", palette.overlay2),
        ("%%SUBTEXT0%%", palette.subtext0),
        ("%%SUBTEXT1%%", palette.subtext1),
        ("%%TEXT%%", palette.text),
        ("%%LAVENDER%%", palette.lavender),
        ("%%BLUE%%", palette.blue),
        ("%%MAUVE%%", palette.mauve),
        ("%%RED%%", palette.red),
        ("%%GREEN%%", palette.green),
        ("%%YELLOW%%", palette.yellow),
        ("%%TEAL%%", palette.teal),
        ("%%SKY%%", palette.sky),
        ("%%PINK%%", palette.pink),
        ("%%PEACH%%", palette.peach),
    ] {
        css = css.replace(token, value);
    }
    css
}

/// Builds the full stylesheet for `theme`: the light block resolved against
/// the light palette, then the dark block against the dark palette.
pub(crate) fn build_css(theme: ThemeId) -> String {
    let light = palettes::palette(theme, Variant::Light);
    let dark = palettes::palette(theme, Variant::Dark);
    let mut css = render_block(LIGHT_BLOCK, &light);
    css.push('\n');
    css.push_str(&render_block(DARK_BLOCK, &dark));
    css
}

/// Retained handle over the application-wide CSS provider.
///
/// [`load`] returns it so the palette can be re-targeted at runtime; dropping
/// it does not unregister the provider (the display keeps its own reference),
/// but the handle should live as long as the app wants to call [`apply`].
///
/// [`apply`]: ThemeHandle::apply
pub struct ThemeHandle {
    provider: gtk4::CssProvider,
}

impl ThemeHandle {
    /// Rebuilds the stylesheet for `theme` and reloads it into the retained
    /// provider, updating every widget using the application priority.
    pub fn apply(&self, theme: ThemeId) {
        self.provider.load_from_string(&build_css(theme));
    }
}

/// Loads the default (Catppuccin) stylesheet for `display` at application
/// priority and returns a handle that can re-target the palette at runtime.
///
/// The provider stays registered (and alive) with the display, matching how
/// the autocomplete panel installs its provider in `wiki_autocomplete.rs`.
pub fn load(display: &gdk::Display) -> ThemeHandle {
    let provider = gtk4::CssProvider::new();

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

    let handle = ThemeHandle { provider };
    // Today's default palette; T5's picker will call `apply` with other ids.
    handle.apply(ThemeId::Catppuccin);
    handle
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_THEMES: [ThemeId; 4] = [
        ThemeId::Catppuccin,
        ThemeId::Dracula,
        ThemeId::Flexoki,
        ThemeId::Wallpaper,
    ];

    #[test]
    fn generated_css_defines_both_variants_with_every_name() {
        for theme in ALL_THEMES {
            let css = build_css(theme);
            assert!(
                css.contains("@media (prefers-color-scheme: light)"),
                "{}: light media block missing",
                theme.as_str()
            );
            assert!(
                css.contains("@media (prefers-color-scheme: dark)"),
                "{}: dark media block missing",
                theme.as_str()
            );
            assert!(css.contains("@define-color theme_base_color @base;"));
            assert!(css.contains("@define-color borders @surface1;"));
            assert!(
                !css.contains("%%"),
                "{}: template token left unsubstituted",
                theme.as_str()
            );
            // 22 palette entries + 39 libadwaita/legacy variables per variant.
            assert_eq!(
                css.matches("@define-color").count(),
                2 * 61,
                "{}: unexpected named-color count",
                theme.as_str()
            );
        }
    }

    #[test]
    fn catppuccin_css_keeps_latte_and_mocha_bases() {
        let css = build_css(ThemeId::Catppuccin);
        assert!(css.contains("#eff1f5"), "Latte base missing");
        assert!(css.contains("#1e1e2e"), "Mocha base missing");
    }
}
