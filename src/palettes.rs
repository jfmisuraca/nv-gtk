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

use std::borrow::Cow;

/// A user-selectable desktop palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemeId {
    Catppuccin,
    Dracula,
    Flexoki,
    Wallpaper,
}

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

    /// The user-facing label shown in the theme picker. Brand names stay
    /// verbatim; only the descriptive `Wallpaper` id is translated, because
    /// the whole UI is Spanish. Presentation only — persistence keeps using
    /// [`as_str`](Self::as_str).
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Catppuccin => "Catppuccin",
            Self::Dracula => "Dracula",
            Self::Flexoki => "Flexoki",
            Self::Wallpaper => "Papel tapiz",
        }
    }

    /// Parses the stored id, falling back to today's default (Catppuccin) for
    /// anything unknown — the single place for "stored id -> active theme".
    pub fn from_persisted(s: &str) -> Self {
        Self::parse(s).unwrap_or(Self::Catppuccin)
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
///
/// Slots are `Cow<'static, str>` rather than `&'static str` so the pywal source
/// (T3) can return runtime-derived colors through this same struct; the named
/// themes below stay `const` by wrapping their literals in [`borrowed`].
#[derive(Debug, Clone)]
pub struct Palette {
    pub base: Cow<'static, str>,
    pub mantle: Cow<'static, str>,
    pub crust: Cow<'static, str>,
    pub surface0: Cow<'static, str>,
    pub surface1: Cow<'static, str>,
    pub surface2: Cow<'static, str>,
    pub overlay0: Cow<'static, str>,
    pub overlay1: Cow<'static, str>,
    pub overlay2: Cow<'static, str>,
    pub subtext0: Cow<'static, str>,
    pub subtext1: Cow<'static, str>,
    pub text: Cow<'static, str>,
    pub lavender: Cow<'static, str>,
    pub blue: Cow<'static, str>,
    pub mauve: Cow<'static, str>,
    pub red: Cow<'static, str>,
    pub green: Cow<'static, str>,
    pub yellow: Cow<'static, str>,
    pub teal: Cow<'static, str>,
    pub sky: Cow<'static, str>,
    pub pink: Cow<'static, str>,
    pub peach: Cow<'static, str>,
}

/// Wraps a hex literal so the named-theme tables stay `const` while `Palette`
/// holds `Cow<'static, str>` for the runtime-derived pywal colors.
const fn borrowed(s: &'static str) -> Cow<'static, str> {
    Cow::Borrowed(s)
}

/// Catppuccin Latte (light), verbatim from the original stylesheet.
const CATPPUCCIN_LATTE: Palette = Palette {
    base: borrowed("#eff1f5"),
    mantle: borrowed("#e6e9ef"),
    crust: borrowed("#dce0e8"),
    surface0: borrowed("#ccd0da"),
    surface1: borrowed("#bcc0cc"),
    surface2: borrowed("#acb0be"),
    overlay0: borrowed("#9ca0b0"),
    overlay1: borrowed("#8c8fa1"),
    overlay2: borrowed("#7c7f93"),
    subtext0: borrowed("#6c6f85"),
    subtext1: borrowed("#5c5f77"),
    text: borrowed("#4c4f69"),
    lavender: borrowed("#7287fd"),
    blue: borrowed("#1e66f5"),
    mauve: borrowed("#8839ef"),
    red: borrowed("#d20f39"),
    green: borrowed("#40a02b"),
    yellow: borrowed("#df8e1d"),
    teal: borrowed("#179299"),
    sky: borrowed("#04a5e5"),
    pink: borrowed("#ea76cb"),
    peach: borrowed("#fe640b"),
};

/// Catppuccin Mocha (dark), verbatim from the original stylesheet.
const CATPPUCCIN_MOCHA: Palette = Palette {
    base: borrowed("#1e1e2e"),
    mantle: borrowed("#181825"),
    crust: borrowed("#11111b"),
    surface0: borrowed("#313244"),
    surface1: borrowed("#45475a"),
    surface2: borrowed("#585b70"),
    overlay0: borrowed("#6c7086"),
    overlay1: borrowed("#7f849c"),
    overlay2: borrowed("#9399b2"),
    subtext0: borrowed("#a6adc8"),
    subtext1: borrowed("#bac2de"),
    text: borrowed("#cdd6f4"),
    lavender: borrowed("#b4befe"),
    blue: borrowed("#89b4fa"),
    mauve: borrowed("#cba6f7"),
    red: borrowed("#f38ba8"),
    green: borrowed("#a6e3a1"),
    yellow: borrowed("#f9e2af"),
    teal: borrowed("#94e2d5"),
    sky: borrowed("#89dceb"),
    pink: borrowed("#f5c2e7"),
    peach: borrowed("#fab387"),
};

/// Dracula (dark). The spec defines backgrounds, `selection`, `comment`, `fg`
/// and the accent hues, but no elevation ramp: `overlay0..2` and
/// `subtext0..1` are blended `base -> text` at 0.44/0.55/0.67/0.78/0.89,
/// mirroring Catppuccin's own relative elevation curve. Dracula has no blue,
/// so `blue`/`teal`/`sky` alias `cyan`, and `lavender`/`mauve` alias `purple`.
const DRACULA: Palette = Palette {
    base: borrowed("#282a36"),
    mantle: borrowed("#21222c"),
    crust: borrowed("#191a21"),
    surface0: borrowed("#343746"),
    surface1: borrowed("#424450"),
    surface2: borrowed("#44475a"),
    overlay0: borrowed("#848589"),
    overlay1: borrowed("#9a9b9d"),
    overlay2: borrowed("#b3b4b4"),
    subtext0: borrowed("#cacbc9"),
    subtext1: borrowed("#e1e1dd"),
    text: borrowed("#f8f8f2"),
    lavender: borrowed("#bd93f9"),
    blue: borrowed("#8be9fd"),
    mauve: borrowed("#bd93f9"),
    red: borrowed("#ff5555"),
    green: borrowed("#50fa7b"),
    yellow: borrowed("#f1fa8c"),
    teal: borrowed("#8be9fd"),
    sky: borrowed("#8be9fd"),
    pink: borrowed("#ff79c6"),
    peach: borrowed("#ffb86c"),
};

/// Alucard (Dracula light). Same derivation as [`DRACULA`]: the elevation
/// steps are blended `base -> text` at 0.44/0.55/0.67/0.78/0.89 (here that
/// darkens, since `base` is light). No blue in the spec, so `blue`/`teal`/`sky`
/// alias `cyan` and `lavender`/`mauve` alias `purple`.
const ALUCARD: Palette = Palette {
    base: borrowed("#fffbeb"),
    mantle: borrowed("#efeddc"),
    crust: borrowed("#ece9df"),
    surface0: borrowed("#dedccf"),
    surface1: borrowed("#ceccc0"),
    surface2: borrowed("#bcbab3"),
    overlay0: borrowed("#9c9a91"),
    overlay1: borrowed("#84827b"),
    overlay2: borrowed("#696862"),
    subtext0: borrowed("#504f4c"),
    subtext1: borrowed("#383735"),
    text: borrowed("#1f1f1f"),
    lavender: borrowed("#644ac9"),
    blue: borrowed("#036a96"),
    mauve: borrowed("#644ac9"),
    red: borrowed("#cb3a2a"),
    green: borrowed("#14710a"),
    yellow: borrowed("#846e15"),
    teal: borrowed("#036a96"),
    sky: borrowed("#036a96"),
    pink: borrowed("#a3144d"),
    peach: borrowed("#a34d14"),
};

/// Flexoki dark. The 9-step base ramp supplies the neutrals from `mantle`
/// through `subtext0` (Flexoki's near-black surfaces sit *above* `base`, its
/// own convention); `subtext1` is the only derived step, blended
/// `base -> text` at 0.86. The spec names `cyan`/`purple`/`magenta`, mapped
/// onto the stylesheet's `teal`/`sky`, `lavender`/`mauve` and `pink`.
const FLEXOKI_DARK: Palette = Palette {
    base: borrowed("#100f0f"),
    mantle: borrowed("#1c1b1a"),
    crust: borrowed("#282726"),
    surface0: borrowed("#343331"),
    surface1: borrowed("#403e3c"),
    surface2: borrowed("#575653"),
    overlay0: borrowed("#6f6e69"),
    overlay1: borrowed("#878580"),
    overlay2: borrowed("#9f9d96"),
    subtext0: borrowed("#b7b5ac"),
    subtext1: borrowed("#d2d0c7"),
    text: borrowed("#f2f0e5"),
    lavender: borrowed("#5e409d"),
    blue: borrowed("#205ea6"),
    mauve: borrowed("#5e409d"),
    red: borrowed("#af3029"),
    green: borrowed("#66800b"),
    yellow: borrowed("#ad8301"),
    teal: borrowed("#24837b"),
    sky: borrowed("#24837b"),
    pink: borrowed("#a02f6f"),
    peach: borrowed("#bc5215"),
};

/// Flexoki light. The 9-step base ramp supplies the neutrals from `mantle`
/// through `subtext0`; `subtext1` is the only derived step, blended
/// `base -> text` (darkening) at 0.86. Accent aliasing matches
/// [`FLEXOKI_DARK`].
const FLEXOKI_LIGHT: Palette = Palette {
    base: borrowed("#fffcf0"),
    mantle: borrowed("#f2f0e5"),
    crust: borrowed("#e6e4d9"),
    surface0: borrowed("#dad8ce"),
    surface1: borrowed("#cecdc3"),
    surface2: borrowed("#b7b5ac"),
    overlay0: borrowed("#9f9d96"),
    overlay1: borrowed("#878580"),
    overlay2: borrowed("#6f6e69"),
    subtext0: borrowed("#575653"),
    subtext1: borrowed("#31302e"),
    text: borrowed("#100f0f"),
    lavender: borrowed("#8b7ec8"),
    blue: borrowed("#4385be"),
    mauve: borrowed("#8b7ec8"),
    red: borrowed("#d14d41"),
    green: borrowed("#879a39"),
    yellow: borrowed("#d0a215"),
    teal: borrowed("#3aa99f"),
    sky: borrowed("#3aa99f"),
    pink: borrowed("#ce5d97"),
    peach: borrowed("#da702c"),
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
                let ratio = contrast_ratio(&p.text, &p.base);
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

    #[test]
    fn display_names_are_non_empty_and_distinct() {
        let names: Vec<&str> = ALL_THEMES.iter().map(|theme| theme.display_name()).collect();
        for name in &names {
            assert!(!name.is_empty(), "display name must not be empty");
        }
        for (i, a) in names.iter().enumerate() {
            for b in &names[i + 1..] {
                assert_ne!(a, b, "display names must be pairwise distinct");
            }
        }
    }

    #[test]
    fn from_persisted_maps_valid_ids_and_falls_back_to_catppuccin() {
        for theme in ALL_THEMES {
            assert_eq!(ThemeId::from_persisted(theme.as_str()), theme);
        }
        for garbage in ["", "solarized", "Catppuccin", "dracula "] {
            assert_eq!(
                ThemeId::from_persisted(garbage),
                ThemeId::Catppuccin,
                "from_persisted({garbage:?}) must fall back to Catppuccin"
            );
        }
    }
}
