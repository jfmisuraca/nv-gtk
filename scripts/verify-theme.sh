#!/bin/sh
# Verify the desktop Catppuccin theme (src/theme.rs) on real rendered pixels.
#
# Runs the app headless on Xvfb and asserts the window background is the
# Latte base in light mode and the Mocha base in dark mode, then checks that
# every color alias another stylesheet uses is defined in BOTH variants.
#
# Why the isolation below matters (learned the hard way):
#   libadwaita's AdwStyleManager, not GtkSettings, decides the effective color
#   scheme. It prefers the org.freedesktop.appearance portal, which proxies the
#   REAL session preference -- so a plain headless run always reports the
#   developer's own theme and a light-mode test silently passes as dark.
#   ADW_DISABLE_PORTAL=1 plus a keyfile GSettings backend pins the preference.
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

screenshot() {
  # $1 = color-scheme preference, $2 = output png
  pref="$1"; out="$2"
  home="$TMP/home-$pref"
  mkdir -p "$home/.config/glib-2.0/settings"
  printf "[org/gnome/desktop/interface]\ncolor-scheme='%s'\n" "$pref" \
    >"$home/.config/glib-2.0/settings/keyfile"

  HOME="$home" XDG_CONFIG_HOME="$home/.config" \
  GSETTINGS_BACKEND=keyfile ADW_DISABLE_PORTAL=1 \
    xvfb-run -a --server-args="-screen 0 1400x1000x24" sh -c '
      "$1" >"$3" 2>&1 &
      pid=$!
      sleep 6
      import -window root "$2" 2>/dev/null
      kill "$pid" 2>/dev/null
    ' sh "$BIN" "$out" "$out.log"
}

assert_variant() {
  variant="$1" pref="$2" base="$3" extra="$4" shot="$TMP/$1.png"
  screenshot "$pref" "$shot"
  [ -f "$shot" ] || { echo "FAIL $variant: no screenshot"; exit 1; }

  center="$("$MAGICK" "$shot" -format '%[hex:p{700,500}]' info:)"
  if [ "$center" != "$base" ]; then
    echo "FAIL $variant: window background is #$center, expected #$base"
    exit 1
  fi
  if ! "$MAGICK" "$shot" -format %c -depth 8 histogram:info:- \
       | grep -qi "$extra"; then
    echo "FAIL $variant: #$extra (theme surface color) not present"
    exit 1
  fi
  echo "PASS $variant: background #$center, surface #$extra"
}

assert_variant light prefer-light EFF1F5 E6E9EF
assert_variant dark  prefer-dark  1E1E2E 181825

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
