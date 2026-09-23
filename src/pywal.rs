//! Desktop wallpaper color source, read from pywal's cache output.
//!
//! pywal writes two files under its cache directory (by default
//! `~/.cache/wal`): `colors.json` (the full export: `special.background`,
//! `special.foreground` plus `colors.color0..15`, alongside extra keys such as
//! `wallpaper` and `alpha`) and `colors` (a plain 16-line file holding just
//! the `color0..15` hex values, one per line).
//!
//! # Parsing is schema-specific, by design
//!
//! There is deliberately no JSON dependency and no `regex`: pywal's output
//! shape is fixed, so [`load_from_dir`] uses a small deterministic extractor
//! instead of a general parser. For `colors.json` it searches for the exact
//! quoted key tokens — `"background"`, `"foreground"`, `"color0"` …
//! `"color15"` — and reads the `"#rrggbb"` string that follows the next `:`.
//! Matching the token *including its closing quote* is what keeps `"color1"`
//! from matching `"color10"`. Extra keys pywal writes (`wallpaper`, `alpha`,
//! `cursor`, …) are ignored; key order does not matter.
//!
//! The contract is all-or-nothing: `None` when the file that exists is
//! missing, malformed or partial — i.e. any required key is absent or any
//! value is not `#rrggbb`. Accepted values may use upper- or lowercase hex
//! and are normalized to lowercase in the returned [`Palette`].
//!
//! When `colors.json` does not exist, [`load_from_dir`] falls back to the
//! `colors` file, which must hold exactly 16 `#rrggbb` lines (a blank trailing
//! line is tolerated). That file carries no `special` block, so by convention
//! `base = color0` and `text = color7`: in ANSI terms color0 is the background
//! slot and color7 the normal foreground. A malformed `colors.json` never
//! falls back to `colors`: its presence means pywal ran and wrote something
//! unexpected, which must surface as `None` rather than silently resolve to
//! stale colors.
//!
//! # Mapping to `Palette`
//!
//! - `base`/`text` come from `background`/`foreground`, or from
//!   `color0`/`color7` on the `colors`-file fallback path.
//! - `mix(a, b, t)` is per-channel sRGB linear interpolation,
//!   `round(a + (b - a) * t)`, rendered as lowercase `#rrggbb`.
//! - The elevation ramp blends `base` toward `text`: `surface0` at t=0.20,
//!   `surface1` at t=0.32, `surface2` at t=0.44, `overlay0` at t=0.55,
//!   `overlay1` at t=0.67, `overlay2` at t=0.78, `subtext0` at t=0.88 and
//!   `subtext1` at t=0.94 (each `mix(base, text, t)`).
//! - `mantle` is `base` scaled toward black by 0.06 and `crust` toward black
//!   by 0.12 (per channel `round(c * (1 - t))`). The Catppuccin family keeps
//!   mantle/crust below base in luminance in both variants, and the pywal
//!   palette follows that same direction so the header bar and borders keep
//!   their relative depth against the window background.
//! - Accents map straight across the ANSI slots: `red = color1`,
//!   `green = color2`, `yellow = color3`, `blue = color4`, `mauve = color5`,
//!   `teal = color6`, `lavender = color12`, `sky = color14`, `pink = color13`,
//!   `peach = color9`.
//!
//! There is intentionally no contrast or sanity gate: the contract is only
//! "missing, malformed or partial -> `None`". A low-contrast wallpaper stays
//! an accepted, documented risk (see `odd/tasks/theme-palettes.md`); the
//! caller falls back to Catppuccin on `None`.

use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};

use crate::palettes::Palette;

/// Reads the pywal palette from the default cache directory.
///
/// That is `$XDG_CACHE_HOME/wal` when `XDG_CACHE_HOME` is set, otherwise
/// `$HOME/.cache/wal`. Returns `None` when neither variable is set, or when
/// the directory holds no usable pywal output (see [`load_from_dir`]).
#[allow(dead_code)] // T5 wires this into the picker; nothing calls it yet.
pub fn load() -> Option<Palette> {
    let dir = match std::env::var_os("XDG_CACHE_HOME") {
        Some(cache) if !cache.is_empty() => PathBuf::from(cache).join("wal"),
        _ => {
            let home = std::env::var_os("HOME").filter(|home| !home.is_empty())?;
            PathBuf::from(home).join(".cache/wal")
        }
    };
    load_from_dir(&dir)
}

/// Reads the pywal palette from an explicit cache directory.
///
/// Prefers `dir/colors.json`; when that file does not exist, falls back to
/// `dir/colors`. Returns `None` when neither exists, or when the file that
/// exists is malformed or partial — a broken `colors.json` never falls back
/// to `colors`.
#[allow(dead_code)] // T5 wires this into the picker; only tests call it today.
pub fn load_from_dir(dir: &Path) -> Option<Palette> {
    match fs::read_to_string(dir.join("colors.json")) {
        Ok(text) => from_colors_json(&text),
        Err(_) => {
            let text = fs::read_to_string(dir.join("colors")).ok()?;
            from_colors_file(&text)
        }
    }
}

/// Builds a [`Palette`] from parsed `colors.json` contents: `background`,
/// `foreground` and `color0..15`. `None` if any required key is missing or
/// any value is not `#rrggbb`.
fn from_colors_json(text: &str) -> Option<Palette> {
    let base = extract_hex(text, "background")?;
    let foreground = extract_hex(text, "foreground")?;
    let mut colors = Vec::with_capacity(16);
    for index in 0..16 {
        colors.push(extract_hex(text, &format!("color{index}"))?);
    }
    Some(build_palette(&base, &foreground, &colors))
}

/// Builds a [`Palette`] from parsed `colors`-file contents: exactly 16
/// `#rrggbb` lines (a blank trailing line is tolerated). `None` on any other
/// shape. With no `special` block, `base = color0` and `text = color7` by the
/// convention documented in the module docs.
fn from_colors_file(text: &str) -> Option<Palette> {
    let mut lines: Vec<&str> = text.lines().collect();
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    if lines.len() != 16 {
        return None;
    }
    let mut colors = Vec::with_capacity(16);
    for line in lines {
        colors.push(normalize_hex(line.trim())?);
    }
    let base = colors[0].clone();
    let foreground = colors[7].clone();
    Some(build_palette(&base, &foreground, &colors))
}

/// Extracts the hex value stored under the exact quoted JSON key `key`.
///
/// Finds `"key"` *including its closing quote* — so `"color1"` never matches
/// `"color10"` — then reads the quoted string after the next `:`. Returns the
/// value normalized to lowercase, or `None` when the key is absent or the
/// value is not `#rrggbb`.
fn extract_hex(json: &str, key: &str) -> Option<String> {
    let token = format!("\"{key}\"");
    json.match_indices(&token).find_map(|(start, _)| {
        let after_key = json[start + token.len()..].trim_start();
        let quoted = after_key
            .strip_prefix(':')?
            .trim_start()
            .strip_prefix('"')?;
        let end = quoted.find('"')?;
        normalize_hex(&quoted[..end])
    })
}

/// Validates `#rrggbb` (either hex case) and normalizes it to lowercase.
fn normalize_hex(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    if bytes.len() != 7 || bytes[0] != b'#' {
        return None;
    }
    if !bytes[1..].iter().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some(raw.to_ascii_lowercase())
}

/// Parses an already-validated `#rrggbb` string into its channels.
fn rgb(hex: &str) -> (u8, u8, u8) {
    let channel = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&hex[range], 16).expect("pywal hex already validated")
    };
    (channel(1..3), channel(3..5), channel(5..7))
}

/// Per-channel sRGB linear interpolation, `round(a + (b - a) * t)`, rendered
/// as lowercase `#rrggbb`.
fn mix(a: &str, b: &str, t: f64) -> String {
    let (ar, ag, ab) = rgb(a);
    let (br, bg, bb) = rgb(b);
    let channel = |from: u8, to: u8| {
        (f64::from(from) + (f64::from(to) - f64::from(from)) * t).round() as u8
    };
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(ar, br),
        channel(ag, bg),
        channel(ab, bb)
    )
}

/// Scales `base` toward black by `t` (per channel `round(c * (1 - t))`),
/// rendered as lowercase `#rrggbb`.
fn shade(base: &str, t: f64) -> String {
    let (r, g, b) = rgb(base);
    let channel = |c: u8| (f64::from(c) * (1.0 - t)).round() as u8;
    format!("#{:02x}{:02x}{:02x}", channel(r), channel(g), channel(b))
}

/// Maps `base`/`text` plus the 16 ANSI colors onto the 22 [`Palette`] slots
/// using the blend rule documented in the module docs.
fn build_palette(base: &str, text: &str, colors: &[String]) -> Palette {
    let owned = |hex: String| Cow::Owned(hex);
    Palette {
        base: owned(base.to_owned()),
        mantle: owned(shade(base, 0.06)),
        crust: owned(shade(base, 0.12)),
        surface0: owned(mix(base, text, 0.20)),
        surface1: owned(mix(base, text, 0.32)),
        surface2: owned(mix(base, text, 0.44)),
        overlay0: owned(mix(base, text, 0.55)),
        overlay1: owned(mix(base, text, 0.67)),
        overlay2: owned(mix(base, text, 0.78)),
        subtext0: owned(mix(base, text, 0.88)),
        subtext1: owned(mix(base, text, 0.94)),
        text: owned(text.to_owned()),
        red: owned(colors[1].clone()),
        green: owned(colors[2].clone()),
        yellow: owned(colors[3].clone()),
        blue: owned(colors[4].clone()),
        mauve: owned(colors[5].clone()),
        teal: owned(colors[6].clone()),
        lavender: owned(colors[12].clone()),
        sky: owned(colors[14].clone()),
        pink: owned(colors[13].clone()),
        peach: owned(colors[9].clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn slots(p: &Palette) -> [(&str, &str); 22] {
        [
            ("base", p.base.as_ref()),
            ("mantle", p.mantle.as_ref()),
            ("crust", p.crust.as_ref()),
            ("surface0", p.surface0.as_ref()),
            ("surface1", p.surface1.as_ref()),
            ("surface2", p.surface2.as_ref()),
            ("overlay0", p.overlay0.as_ref()),
            ("overlay1", p.overlay1.as_ref()),
            ("overlay2", p.overlay2.as_ref()),
            ("subtext0", p.subtext0.as_ref()),
            ("subtext1", p.subtext1.as_ref()),
            ("text", p.text.as_ref()),
            ("lavender", p.lavender.as_ref()),
            ("blue", p.blue.as_ref()),
            ("mauve", p.mauve.as_ref()),
            ("red", p.red.as_ref()),
            ("green", p.green.as_ref()),
            ("yellow", p.yellow.as_ref()),
            ("teal", p.teal.as_ref()),
            ("sky", p.sky.as_ref()),
            ("pink", p.pink.as_ref()),
            ("peach", p.peach.as_ref()),
        ]
    }

    fn is_lowercase_hex(s: &str) -> bool {
        let bytes = s.as_bytes();
        bytes.len() == 7
            && bytes[0] == b'#'
            && bytes[1..]
                .iter()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    }

    #[test]
    fn valid_colors_json_maps_background_and_foreground() {
        let palette = load_from_dir(&fixture("pywal")).expect("valid fixture parses");
        assert_eq!(palette.base.as_ref(), "#1a1b26");
        // The fixture declares the foreground uppercase; output is normalized.
        assert_eq!(palette.text.as_ref(), "#c0caf5");
        for (name, value) in slots(&palette) {
            assert!(
                is_lowercase_hex(value),
                "slot {name} = {value} is not lowercase #rrggbb"
            );
        }
        // Spot-checks against the documented arithmetic for base #1a1b26
        // (26, 27, 38) and text #c0caf5 (192, 202, 245):
        // mantle = base * 0.94 -> (24, 25, 36).
        assert_eq!(palette.mantle.as_ref(), "#181924");
        // crust = base * 0.88 -> (23, 24, 33).
        assert_eq!(palette.crust.as_ref(), "#171821");
        // surface0 = mix(base, text, 0.20) -> (59.2, 62.0, 79.4) -> (59, 62, 79).
        assert_eq!(palette.surface0.as_ref(), "#3b3e4f");
        // color1 is declared uppercase in the fixture; accents normalize too.
        assert_eq!(palette.red.as_ref(), "#f7768e");
        assert_eq!(palette.peach.as_ref(), "#f7768e");
        assert_eq!(palette.blue.as_ref(), "#7aa2f7");
    }

    #[test]
    fn colors_file_fallback_uses_color0_and_color7() {
        let palette =
            load_from_dir(&fixture("pywal_colors_only")).expect("16-line fixture parses");
        // color0 is declared uppercase in the fixture; output is normalized.
        assert_eq!(palette.base.as_ref(), "#2e3440");
        assert_eq!(palette.text.as_ref(), "#e5e9f0");
        for (name, value) in slots(&palette) {
            assert!(
                is_lowercase_hex(value),
                "slot {name} = {value} is not lowercase #rrggbb"
            );
        }
    }

    #[test]
    fn malformed_colors_json_returns_none_without_fallback() {
        // The fixture directory also holds a *valid* `colors` file: `None`
        // here proves a broken `colors.json` never silently falls back to it.
        assert!(load_from_dir(&fixture("pywal_malformed")).is_none());
    }

    #[test]
    fn missing_directory_returns_none() {
        assert!(load_from_dir(&fixture("does-not-exist")).is_none());
    }

    #[test]
    fn colors_json_is_preferred_over_colors_file() {
        let palette =
            load_from_dir(&fixture("pywal_both")).expect("complete colors.json parses");
        // `colors.json` wins over the `colors` file (whose every line is
        // #abcdef): base is the declared background, normalized to lowercase.
        assert_eq!(palette.base.as_ref(), "#222222");
        assert_eq!(palette.text.as_ref(), "#eeeeee");
    }

    #[test]
    fn extractor_does_not_confuse_color1_with_color10() {
        // The valid fixture carries both color1 and color10..15; an extractor
        // matching `"color1` without its closing quote would read color10's
        // value here instead.
        assert_eq!(extract_hex("{\"color1\": \"#aaaaaa\", \"color10\": \"#bbbbbb\"}", "color1"), Some("#aaaaaa".to_string()));
        assert_eq!(extract_hex("{\"color1\": \"#aaaaaa\", \"color10\": \"#bbbbbb\"}", "color10"), Some("#bbbbbb".to_string()));
    }
}
