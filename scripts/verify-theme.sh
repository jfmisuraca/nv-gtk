#!/bin/sh
# Verify the desktop theme palettes (src/palettes.rs, src/theme.rs) on real rendered pixels.
#
# Runs the app headless on Xvfb and asserts the window background plus the
# theme surface (mantle) color for every (theme, color-scheme) pair in the
# matrix below -- Catppuccin, Dracula (whose light variant is Alucard) and
# Flexoki, each in light and dark -- plus pywal runs proving a
# wallpaper-derived theme ignores the OS variant. Then checks that every
# color alias another stylesheet uses is defined in BOTH variants.
#
# The persisted theme id is written into the run's own config file before
# launch ($XDG_CONFIG_HOME/nv-gtk/config.json), because the app reads
# Config::theme at startup and takes no theme flag on the command line.
#
# Why the isolation below matters (learned the hard way):
#   libadwaita's AdwStyleManager, not GtkSettings, decides the effective color
#   scheme. It prefers the org.freedesktop.appearance portal, which proxies the
#   REAL session preference -- so a plain headless run always reports the
#   developer's own theme and a light-mode test silently passes as dark.
#   ADW_DISABLE_PORTAL=1 plus a keyfile GSettings backend pins the preference.
#   Each run also gets its own HOME, XDG_CONFIG_HOME and XDG_CACHE_HOME, so a
#   developer's real config -- and, critically, their real pywal cache
#   (~/.cache/wal or $XDG_CACHE_HOME/wal, which the wallpaper theme reads) --
#   can never leak into a test. A named-theme run must not see pywal output,
#   and a pywal run must see only the fixture this script writes.
set -eu

cd "$(dirname "$0")/.."
REPO="$PWD"
BIN="$REPO/target/debug/nv-gtk"

[ -x "$BIN" ] || { echo "Build it first: cargo build -p nv-gtk --bin nv-gtk" >&2; exit 1; }
command -v Xvfb >/dev/null || { echo "Xvfb is required" >&2; exit 1; }
if command -v magick >/dev/null; then MAGICK=magick
elif command -v convert >/dev/null; then MAGICK=convert
else echo "ImageMagick (magick/convert) is required" >&2; exit 1
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

write_config() {
  # $1 = isolated home, $2 = persisted theme id
  home="$1"; theme="$2"
  mkdir -p "$home/.config/nv-gtk" "$home/Notes"
  printf '{"notes_dir":"%s/Notes","default_extension":"md","auto_save_ms":300,"theme":"%s"}' \
    "$home" "$theme" >"$home/.config/nv-gtk/config.json"
}

write_pywal_fixture() {
  # $1 = isolated home. Writes a pywal colors.json with a background distinct
  # from every named-theme base, so the assertion cannot pass vacuously.
  home="$1"
  mkdir -p "$home/.cache/wal"
  cat >"$home/.cache/wal/colors.json" <<'EOF'
{
    "wallpaper": "file:///tmp/verify-theme-wallpaper.jpg",
    "alpha": "100",
    "special": {
        "background": "#1a1b26",
        "foreground": "#c0caf5",
        "cursor": "#c0caf5"
    },
    "colors": {
        "color0": "#15161e",
        "color1": "#f7768e",
        "color2": "#9ece6a",
        "color3": "#e0af68",
        "color4": "#7aa2f7",
        "color5": "#bb9af7",
        "color6": "#7dcfff",
        "color7": "#a9b1d6",
        "color8": "#414868",
        "color9": "#f7768e",
        "color10": "#9ece6a",
        "color11": "#e0af68",
        "color12": "#7aa2f7",
        "color13": "#bb9af7",
        "color14": "#7dcfff",
        "color15": "#c0caf5"
    }
}
EOF
}

screenshot() {
  # $1 = persisted theme id, $2 = color-scheme preference, $3 = output png
  theme="$1"; pref="$2"; out="$3"
  home="$TMP/home-$theme-$pref"
  mkdir -p "$home/.config/glib-2.0/settings"
  printf "[org/gnome/desktop/interface]\ncolor-scheme='%s'\n" "$pref" \
    >"$home/.config/glib-2.0/settings/keyfile"
  write_config "$home" "$theme"

  HOME="$home" XDG_CONFIG_HOME="$home/.config" XDG_CACHE_HOME="$home/.cache" \
  GSETTINGS_BACKEND=keyfile ADW_DISABLE_PORTAL=1 \
    xvfb-run -a --server-args="-screen 0 1400x1000x24" sh -c '
      "$1" >"$3" 2>&1 &
      pid=$!
      sleep 6
      import -window root "$2" 2>/dev/null
      kill "$pid" 2>/dev/null
    ' sh "$BIN" "$out" "$out.log"
}

assert_theme_case() {
  # $1 = label, $2 = theme id, $3 = pref, $4 = expected background, $5 = expected surface
  label="$1"; theme="$2"; pref="$3"; base="$4"; extra="$5"
  shot="$TMP/$(echo "$label" | tr '/' '-').png"
  screenshot "$theme" "$pref" "$shot"
  [ -f "$shot" ] || { echo "FAIL $label: no screenshot"; exit 1; }

  center="$("$MAGICK" "$shot" -format '%[hex:p{700,500}]' info:)"
  if [ "$center" != "$base" ]; then
    echo "FAIL $label: window background is #$center, expected #$base"
    exit 1
  fi
  if ! "$MAGICK" "$shot" -format %c -depth 8 histogram:info:- \
       | grep -qi "$extra"; then
    echo "FAIL $label: #$extra (theme surface color) not present"
    exit 1
  fi
  echo "PASS $label: background #$center, surface #$extra"
}

assert_pywal_case() {
  # $1 = label, $2 = pref. The wallpaper theme must render the pywal
  # background under EITHER preference: end-to-end proof that it ignores the
  # OS variant. The expected mantle is base #1a1b26 shaded toward black by
  # 0.06 (the documented base * 0.94 rule) = #181924.
  label="$1"; pref="$2"
  home="$TMP/home-wallpaper-$pref"
  mkdir -p "$home/.config/glib-2.0/settings"
  printf "[org/gnome/desktop/interface]\ncolor-scheme='%s'\n" "$pref" \
    >"$home/.config/glib-2.0/settings/keyfile"
  write_config "$home" "wallpaper"
  write_pywal_fixture "$home"
  shot="$TMP/$(echo "$label" | tr '/' '-').png"

  HOME="$home" XDG_CONFIG_HOME="$home/.config" XDG_CACHE_HOME="$home/.cache" \
  GSETTINGS_BACKEND=keyfile ADW_DISABLE_PORTAL=1 \
    xvfb-run -a --server-args="-screen 0 1400x1000x24" sh -c '
      "$1" >"$3" 2>&1 &
      pid=$!
      sleep 6
      import -window root "$2" 2>/dev/null
      kill "$pid" 2>/dev/null
    ' sh "$BIN" "$shot" "$shot.log"
  [ -f "$shot" ] || { echo "FAIL $label: no screenshot"; exit 1; }

  center="$("$MAGICK" "$shot" -format '%[hex:p{700,500}]' info:)"
  if [ "$center" != "1A1B26" ]; then
    echo "FAIL $label: window background is #$center, expected #1A1B26"
    exit 1
  fi
  # The derived mantle is asserted strictly: it rendered in the histogram for
  # this palette, so a background-only check would be weaker than what the
  # harness can prove.
  if ! "$MAGICK" "$shot" -format %c -depth 8 histogram:info:- \
       | grep -qi "181924"; then
    echo "FAIL $label: #181924 (pywal mantle color) not present"
    exit 1
  fi
  echo "PASS $label: background #$center, surface #181924"
}

assert_theme_case catppuccin/light catppuccin prefer-light EFF1F5 E6E9EF
assert_theme_case catppuccin/dark  catppuccin prefer-dark  1E1E2E 181825
assert_theme_case dracula/light    dracula    prefer-light FFFBEB EFEDDC
assert_theme_case dracula/dark     dracula    prefer-dark  282A36 21222C
assert_theme_case flexoki/light    flexoki    prefer-light FFFCF0 F2F0E5
assert_theme_case flexoki/dark     flexoki    prefer-dark  100F0F 1C1B1A

assert_pywal_case wallpaper/light prefer-light
assert_pywal_case wallpaper/dark  prefer-dark

# Alias parity: a color defined in one variant only would leave the autocomplete
# panel (which uses @theme_base_color and @borders) unstyled in that variant.
defines() {
  awk "/@media \\(prefers-color-scheme: $1\\)/,/^}/" src/theme.rs \
    | sed -n 's/.*@define-color \([a-z0-9-]*\).*/\1/p' | sort -u
}
defines light >"$TMP/light.colors"
defines dark  >"$TMP/dark.colors"
if ! diff -q "$TMP/light.colors" "$TMP/dark.colors" >/dev/null; then
  echo "FAIL alias parity:"; diff "$TMP/light.colors" "$TMP/dark.colors"; exit 1
fi
echo "PASS alias parity: $(wc -l <"$TMP/light.colors") colors defined in both variants"

for alias in $(grep -o '@[a-z-]*' src/wiki_autocomplete.rs | sed 's/@//' | sort -u); do
  if ! grep -qx "$alias" "$TMP/light.colors"; then
    echo "FAIL autocomplete uses undefined @$alias"; exit 1
  fi
done
echo "PASS autocomplete aliases resolve in both variants"
