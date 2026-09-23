//! Color palettes for the desktop themes.
//!
//! A [`Palette`] holds the 22 slots the desktop stylesheet defines (the task
//! brief said 21, but the stylesheet enumerates 22 names: 12 neutrals plus 10
//! accents). [`ThemeId`] names the user-selectable palettes and [`Variant`]
//! picks the light or dark rendering; [`palette`] resolves the pair.
//!
//! Hex values are authoritative: Catppuccin Latte/Mocha are lifted verbatim
//! from the original hardcoded stylesheet, Dracula/Alucard and Flexoki come
//! from their published specs (see `odd/tasks/theme-palettes.md`). Where a
//! theme does not define an explicit elevation step, the slot is derived by
//! blending `base` toward `text` at a fixed ratio, documented in a comment.
//! Accent slots the theme lacks are aliased to that theme's nearest defined
//! hue; no slot ever borrows another theme's colors.

/// A user-selectable desktop palette.
///
/// `Dracula`, `Flexoki` and `Wallpaper` are resolved options from T1 onward;
/// the pywal source (T3) and the picker wiring (T5) construct them later, which
/// is why the whole vocabulary is declared now.
#[allow(dead_code)] // consumed by T3 (pywal) and T5 (picker), not by this task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeId {
    Catppuccin,
    Dracula,
    Flexoki,
    Wallpaper,
}

#[allow(dead_code)] // `parse`/`as_str` back the persistence layer added in T4.
impl ThemeId {
    /// Parses the persisted lowercase identifier, or `None` for anything else.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "catppuccin" => Some(Self::Catppuccin),
            "dracula" => Some(Self::Dracula),
            "flexoki" => Some(Self::Flexoki),
            "wallpaper" => Some(Self::Wallpaper),
            _ => None,
        }
    }

    /// The stable lowercase identifier used for persistence.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Catppuccin => "catppuccin",
            Self::Dracula => "dracula",
            Self::Flexoki => "flexoki",
            Self::Wallpaper => "wallpaper",
        }
    }
}

/// The light/dark rendering of a palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Light,
    Dark,
}

/// The 22 palette slots the desktop stylesheet defines, named after the
/// vocabulary already used by `src/theme.rs`: the 12 neutrals (`base`,
/// `mantle`, `crust`, `surface0..2`, `overlay0..2`, `subtext0..1`, `text`)
/// and the 10 accents (`lavender`, `blue`, `mauve`, `red`, `green`, `yellow`,
/// `teal`, `sky`, `pink`, `peach`).
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub base: &'static str,
    pub mantle: &'static str,
    pub crust: &'static str,
    pub surface0: &'static str,
    pub surface1: &'static str,
    pub surface2: &'static str,
    pub overlay0: &'static str,
    pub overlay1: &'static str,
    pub overlay2: &'static str,
    pub subtext0: &'static str,
    pub subtext1: &'static str,
    pub text: &'static str,
    pub lavender: &'static str,
    pub blue: &'static str,
    pub mauve: &'static str,
    pub red: &'static str,
    pub green: &'static str,
    pub yellow: &'static str,
    pub teal: &'static str,
    pub sky: &'static str,
    pub pink: &'static str,
    pub peach: &'static str,
}

/// Catppuccin Latte (light), verbatim from the original stylesheet.
const CATPPUCCIN_LATTE: Palette = Palette {
    base: "#eff1f5",
    mantle: "#e6e9ef",
    crust: "#dce0e8",
    surface0: "#ccd0da",
    surface1: "#bcc0cc",
    surface2: "#acb0be",
    overlay0: "#9ca0b0",
    overlay1: "#8c8fa1",
    overlay2: "#7c7f93",
    subtext0: "#6c6f85",
    subtext1: "#5c5f77",
    text: "#4c4f69",
    lavender: "#7287fd",
    blue: "#1e66f5",
    mauve: "#8839ef",
    red: "#d20f39",
    green: "#40a02b",
    yellow: "#df8e1d",
    teal: "#179299",
    sky: "#04a5e5",
    pink: "#ea76cb",
    peach: "#fe640b",
};

/// Catppuccin Mocha (dark), verbatim from the original stylesheet.
const CATPPUCCIN_MOCHA: Palette = Palette {
    base: "#1e1e2e",
    mantle: "#181825",
    crust: "#11111b",
    surface0: "#313244",
    surface1: "#45475a",
    surface2: "#585b70",
    overlay0: "#6c7086",
    overlay1: "#7f849c",
    overlay2: "#9399b2",
    subtext0: "#a6adc8",
    subtext1: "#bac2de",
    text: "#cdd6f4",
    lavender: "#b4befe",
    blue: "#89b4fa",
    mauve: "#cba6f7",
    red: "#f38ba8",
    green: "#a6e3a1",
    yellow: "#f9e2af",
    teal: "#94e2d5",
    sky: "#89dceb",
    pink: "#f5c2e7",
    peach: "#fab387",
};

/// Dracula (dark). The spec defines backgrounds, `selection`, `comment`, `fg`
/// and the accent hues, but no elevation ramp: `overlay0..2` and
/// `subtext0..1` are blended `base -> text` at 0.44/0.55/0.67/0.78/0.89,
/// mirroring Catppuccin's own relative elevation curve. Dracula has no blue,
/// so `blue`/`teal`/`sky` alias `cyan`, and `lavender`/`mauve` alias `purple`.
const DRACULA: Palette = Palette {
    base: "#282a36",
    mantle: "#21222c",
    crust: "#191a21",
    surface0: "#343746",
    surface1: "#424450",
    surface2: "#44475a",
    overlay0: "#848589",
    overlay1: "#9a9b9d",
    overlay2: "#b3b4b4",
    subtext0: "#cacbc9",
    subtext1: "#e1e1dd",
    text: "#f8f8f2",
    lavender: "#bd93f9",
    blue: "#8be9fd",
    mauve: "#bd93f9",
    red: "#ff5555",
    green: "#50fa7b",
    yellow: "#f1fa8c",
    teal: "#8be9fd",
    sky: "#8be9fd",
    pink: "#ff79c6",
    peach: "#ffb86c",
};

/// Alucard (Dracula light). Same derivation as [`DRACULA`]: the elevation
/// steps are blended `base -> text` at 0.44/0.55/0.67/0.78/0.89 (here that
/// darkens, since `base` is light). No blue in the spec, so `blue`/`teal`/`sky`
/// alias `cyan` and `lavender`/`mauve` alias `purple`.
const ALUCARD: Palette = Palette {
    base: "#fffbeb",
    mantle: "#efeddc",
    crust: "#ece9df",
    surface0: "#dedccf",
    surface1: "#ceccc0",
    surface2: "#bcbab3",
    overlay0: "#9c9a91",
    overlay1: "#84827b",
    overlay2: "#696862",
    subtext0: "#504f4c",
    subtext1: "#383735",
    text: "#1f1f1f",
    lavender: "#644ac9",
    blue: "#036a96",
    mauve: "#644ac9",
    red: "#cb3a2a",
    green: "#14710a",
    yellow: "#846e15",
    teal: "#036a96",
    sky: "#036a96",
    pink: "#a3144d",
    peach: "#a34d14",
};

/// Flexoki dark. The 9-step base ramp supplies the neutrals from `mantle`
/// through `subtext0` (Flexoki's near-black surfaces sit *above* `base`, its
/// own convention); `subtext1` is the only derived step, blended
/// `base -> text` at 0.86. The spec names `cyan`/`purple`/`magenta`, mapped
/// onto the stylesheet's `teal`/`sky`, `lavender`/`mauve` and `pink`.
const FLEXOKI_DARK: Palette = Palette {
    base: "#100f0f",
    mantle: "#1c1b1a",
    crust: "#282726",
    surface0: "#343331",
    surface1: "#403e3c",
    surface2: "#575653",
    overlay0: "#6f6e69",
    overlay1: "#878580",
    overlay2: "#9f9d96",
    subtext0: "#b7b5ac",
    subtext1: "#d2d0c7",
    text: "#f2f0e5",
    lavender: "#5e409d",
    blue: "#205ea6",
    mauve: "#5e409d",
    red: "#af3029",
    green: "#66800b",
    yellow: "#ad8301",
    teal: "#24837b",
    sky: "#24837b",
    pink: "#a02f6f",
    peach: "#bc5215",
};

/// Flexoki light. The 9-step base ramp supplies the neutrals from `mantle`
/// through `subtext0`; `subtext1` is the only derived step, blended
/// `base -> text` (darkening) at 0.86. Accent aliasing matches
/// [`FLEXOKI_DARK`].
const FLEXOKI_LIGHT: Palette = Palette {
    base: "#fffcf0",
    mantle: "#f2f0e5",
    crust: "#e6e4d9",
    surface0: "#dad8ce",
    surface1: "#cecdc3",
    surface2: "#b7b5ac",
    overlay0: "#9f9d96",
    overlay1: "#878580",
    overlay2: "#6f6e69",
    subtext0: "#575653",
    subtext1: "#31302e",
    text: "#100f0f",
    lavender: "#8b7ec8",
    blue: "#4385be",
    mauve: "#8b7ec8",
    red: "#d14d41",
    green: "#879a39",
    yellow: "#d0a215",
    teal: "#3aa99f",
    sky: "#3aa99f",
    pink: "#ce5d97",
    peach: "#da702c",
};

/// Resolves the palette for a theme and variant.
///
/// `Wallpaper` resolves to the Catppuccin fallback until the pywal source
/// (T3) is wired in: pywal output is not read here, by design of this task.
pub fn palette(theme: ThemeId, variant: Variant) -> Palette {
    match (theme, variant) {
        (ThemeId::Catppuccin | ThemeId::Wallpaper, Variant::Light) => CATPPUCCIN_LATTE,
        (ThemeId::Catppuccin | ThemeId::Wallpaper, Variant::Dark) => CATPPUCCIN_MOCHA,
        (ThemeId::Dracula, Variant::Light) => ALUCARD,
        (ThemeId::Dracula, Variant::Dark) => DRACULA,
        (ThemeId::Flexoki, Variant::Light) => FLEXOKI_LIGHT,
        (ThemeId::Flexoki, Variant::Dark) => FLEXOKI_DARK,
    }
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
    const BOTH_VARIANTS: [Variant; 2] = [Variant::Light, Variant::Dark];

    fn slots(p: &Palette) -> [(&'static str, &'static str); 22] {
        [
            ("base", p.base),
            ("mantle", p.mantle),
            ("crust", p.crust),
            ("surface0", p.surface0),
            ("surface1", p.surface1),
            ("surface2", p.surface2),
            ("overlay0", p.overlay0),
            ("overlay1", p.overlay1),
            ("overlay2", p.overlay2),
            ("subtext0", p.subtext0),
            ("subtext1", p.subtext1),
            ("text", p.text),
            ("lavender", p.lavender),
            ("blue", p.blue),
            ("mauve", p.mauve),
            ("red", p.red),
            ("green", p.green),
            ("yellow", p.yellow),
            ("teal", p.teal),
            ("sky", p.sky),
            ("pink", p.pink),
            ("peach", p.peach),
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

    /// WCAG 2.x relative luminance for an `#rrggbb` color.
    fn relative_luminance(hex: &str) -> f64 {
        fn channel(byte: u8) -> f64 {
            let c = f64::from(byte) / 255.0;
            if c <= 0.039_28 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        }
        let component = |i: usize| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .unwrap_or_else(|_| panic!("invalid hex component in {hex} at byte {i}"))
        };
        let r = channel(component(1));
        let g = channel(component(3));
        let b = channel(component(5));
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }

    fn contrast_ratio(a: &str, b: &str) -> f64 {
        let (la, lb) = (relative_luminance(a), relative_luminance(b));
        let (lighter, darker) = if la >= lb { (la, lb) } else { (lb, la) };
        (lighter + 0.05) / (darker + 0.05)
    }

    #[test]
    fn every_theme_defines_every_slot_in_both_variants() {
        for theme in ALL_THEMES {
            for variant in BOTH_VARIANTS {
                let p = palette(theme, variant);
                let defined = slots(&p);
                assert_eq!(defined.len(), 22);
                for (name, value) in defined {
                    assert!(
                        !value.is_empty(),
                        "{} {variant:?}: slot {name} is empty",
                        theme.as_str()
                    );
                }
            }
        }
    }

    #[test]
    fn every_slot_is_lowercase_rrggbb() {
        for theme in ALL_THEMES {
            for variant in BOTH_VARIANTS {
                let p = palette(theme, variant);
                for (name, value) in slots(&p) {
                    assert!(
                        is_lowercase_hex(value),
                        "{} {variant:?}: slot {name} = {value} is not lowercase #rrggbb",
                        theme.as_str()
                    );
                }
            }
        }
    }

    #[test]
    fn text_base_contrast_meets_wcag_aa() {
        for theme in [ThemeId::Catppuccin, ThemeId::Dracula, ThemeId::Flexoki] {
            for variant in BOTH_VARIANTS {
                let p = palette(theme, variant);
                let ratio = contrast_ratio(p.text, p.base);
                assert!(
                    ratio >= 4.5,
                    "{} {variant:?}: text/base contrast {ratio:.2} is below the 4.5:1 WCAG AA bar",
                    theme.as_str()
                );
            }
        }
    }

    #[test]
    fn theme_id_round_trips_and_rejects_garbage() {
        for theme in ALL_THEMES {
            assert_eq!(ThemeId::parse(theme.as_str()), Some(theme));
        }
        assert_eq!(ThemeId::parse("Catppuccin"), None);
        assert_eq!(ThemeId::parse("solarized"), None);
        assert_eq!(ThemeId::parse(""), None);
        assert_eq!(ThemeId::parse("dracula "), None);
    }

    #[test]
    fn wallpaper_resolves_to_catppuccin_fallback() {
        for variant in BOTH_VARIANTS {
            let wallpaper = palette(ThemeId::Wallpaper, variant);
            let catppuccin = palette(ThemeId::Catppuccin, variant);
            for ((name, a), (_, b)) in slots(&wallpaper).iter().zip(slots(&catppuccin)) {
                assert_eq!(a, &b, "wallpaper slot {name} diverged from the Catppuccin fallback");
            }
        }
    }
}
