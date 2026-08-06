# nv-gtk

Un clon de [Notational Velocity](http://notational.net/) hecho en Rust con GTK4 y libadwaita: notas en texto plano, búsqueda instantánea (por título y contenido), wiki-links con autocompletado y guardado automático.

## Características

- **Búsqueda difusa** sobre título y contenido de las notas (tipo grep), sin tocar el editor hasta que confirmás con Enter.
- **Wiki-links** (`[[texto]]`) con resaltado visual y autocompletado difuso al escribir `[[`, con preview del contenido de la nota candidata.
- **Guardado automático** (debounced, configurable) y al cambiar de nota o cerrar la ventana.
- **Notas en texto plano** (Markdown por defecto), con nombre de archivo generado automáticamente como timestamp (`YYYYMMDD-HHMM.md`); la lista muestra la primera línea del contenido en vez del nombre del archivo.
- Configuración en `~/.config/nv-gtk/config.json` (carpeta de notas, extensión de archivo, tiempo de auto-guardado).

## Instalación

### Arch Linux

Con el `PKGBUILD` incluido en el repo:

```bash
git clone https://github.com/jfmisuraca/nv-gtk
cd nv-gtk
makepkg -si
```

### Ubuntu

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libgtk-4-dev libadwaita-1-dev

git clone <url-de-tu-repo> nv-gtk
cd nv-gtk
cargo build --release

sudo install -Dm755 target/release/nv-gtk /usr/local/bin/nv-gtk
sudo install -Dm644 nv-gtk.desktop /usr/share/applications/nv-gtk.desktop
```

> `libadwaita-1-dev` requiere Ubuntu 23.10 o superior. En versiones más viejas no está disponible en los repos oficiales.

## Atajos de teclado

### Globales (funcionan desde cualquier parte de la ventana)

| Atajo | Acción |
|---|---|
| `Ctrl+L` / `Ctrl+F` / `Escape` | Enfocar la barra de búsqueda (selecciona todo el texto) |
| `Ctrl+N` | Crear una nota nueva vacía |
| `Ctrl+D` | Eliminar la nota actualmente abierta |
| `Ctrl+J` | Abrir la nota siguiente (debajo) en la lista |
| `Ctrl+K` | Abrir la nota anterior (arriba) en la lista |

### Barra de búsqueda

| Atajo | Acción |
|---|---|
| Escribir | Filtra la lista de notas por título y contenido (no abre ninguna nota todavía) |
| `Enter` | Abre la mejor coincidencia filtrada. Si no hay ninguna, crea una nota nueva con el texto escrito como primera línea |

### Lista de notas

| Atajo | Acción |
|---|---|
| `↑` / `↓` | Mueve la selección visual (no abre la nota) |
| `Enter` / doble clic / clic simple | Abre la nota seleccionada |

### Editor

| Atajo | Acción |
|---|---|
| Escribir `[[` | Abre un popover de autocompletado con búsqueda difusa sobre los títulos/contenido de tus notas, con preview de las primeras líneas |
| `↑` / `↓` (con el popover abierto) | Navega entre las notas candidatas |
| `Enter` / `Tab` (con el popover abierto) | Inserta el link a la nota seleccionada |
| `Escape` (con el popover abierto) | Cancela el autocompletado |
| `Ctrl+Enter` (con el cursor sobre un `[[link]]` ya cerrado) | Pone el texto del link en la barra de búsqueda y filtra |

## Configuración

El archivo de configuración vive en `~/.config/nv-gtk/config.json` y se genera automáticamente la primera vez que corrés la app

- **`notes_dir`**: carpeta donde se guardan las notas.
- **`default_extension`**: extensión con la que se crean los archivos nuevos.
- **`auto_save_ms`**: milisegundos de inactividad antes de guardar automáticamente una nota mientras la editás.

## Desarrollo

```bash
cargo build          # build de desarrollo
cargo build --release
cargo run             # compila (si hace falta) y ejecuta
```
