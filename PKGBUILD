pkgname=nv-gtk
pkgver=0.1.0
pkgrel=1
pkgdesc="Recreación nativa de Notational Velocity para Arch Linux (Rust + GTK4 / Libadwaita)"
arch=('x86_64')
url="https://github.com/notational/nv-gtk"
license=('GPL-3.0-or-later')
depends=('gtk4' 'libadwaita')
makedepends=('cargo')
source=()

build() {
    cd "$srcdir/.."
    cargo build --release --locked
}

package() {
    cd "$srcdir/.."
    install -Dm755 "target/release/nv-gtk" "$pkgdir/usr/bin/nv-gtk"
    install -Dm644 "nv-gtk.desktop" "$pkgdir/usr/share/applications/nv-gtk.desktop"
}
