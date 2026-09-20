# Responsive portrait layout (feature/responsive-portrait)

## Objective

Cuando la ventana es más alta que ancha (`height > width`), la app cambia a un
layout vertical: el buscador arriba de todo y la nota abierta ocupando el resto,
con la lista de notas escondida. Al buscar aparece un panel overlay de resultados
bajo el buscador. En horizontal (w > h) el layout actual de dos paneles no cambia.

## Problem

Hoy el `Paned` es siempre horizontal (posición 300): en ventanas altas y
angostas la columna de búsqueda+lista se come el ancho y el editor queda
inservible.

## Why

Pedido del usuario: "cuando el tamaño de la ventana sea más alto que ancho quiero
que la vista sea la nota abierta y arriba de todo el recuadro de búsqueda".
Aprobado por el usuario con panel de resultados en modo overlay.

## Scope

- `src/window.rs` (único archivo de UI, ~684 líneas). Sin cambios de modelo.

## Constraints

- Reutilizar el patrón de overlay existente (`.nv-autocomplete-panel` de
  wiki-links) para el panel de resultados.
- No re-parentar widgets en runtime: la detección cambia orientación del Paned
  y visibilidad de la lista.
- No romper atajos de teclado (Ctrl+L/F, Ctrl+N, Ctrl+D, Ctrl+J/K, Esc).
- Los artifactos generados (UI copy, código, comentarios) en inglés salvo que se
  extienda un comentario existente en español.

## Tasks

- [x] T1: Switch de layout responsivo
      - Detectar `height > width` con `add_tick_callback` leyendo el allocation
        real (`connect_size_allocate` no existe en gtk4-rs 0.9; el signal
        "size-allocate" no marshalea con `connect_closure`).
      - En vertical: `paned.set_orientation(Vertical)`, ocultar `list_scroll`,
        `set_position(0)`.
      - En horizontal: restaurar `Orientation::Horizontal`, posición 300, lista visible.
      - Estado `is_portrait: Rc<Cell<bool>>` compartido.
      - Evidencia: build+test verdes; smoke test xvfb con resize 500x800 y
        vuelta a 1000x500 vía xdotool sin panic. Commit `7303073`.
- [x] T2: Overlay de resultados de búsqueda en modo vertical
      - Nuevo `ListBox` de resultados + `ScrolledWindow` + panel con clase
        `nv-autocomplete-panel`, agregado al `editor_overlay`.
      - `populate_list` puebla también el overlay (helper `build_note_row`).
      - Mostrar: foco en buscador o query no vacía (solo vertical).
      - Ocultar: Enter con nota elegida, fila activada, Esc, o foco fuera con
        query vacía.
      - `row-activated` del overlay abre la nota (mismo handler que el sidebar).
      - Checks: `cargo build`, `cargo test`.
- [x] T3: Navegación por teclado sobre la lista visible
      - `select_row_by_id`, `select_note_by_id` y `move_list_selection` usan la
        lista activa (`results_list_box` en vertical, `list_box` en horizontal).
      - Ctrl+J/K en vertical abren el overlay si estaba oculto y navegan.
      - Esc en vertical oculta el overlay si está visible (sino, foco al buscador).
      - Checks: `cargo build`, `cargo test`.

## Authorized scope

Rama `feature/responsive-portrait`, solo cambios en `src/window.rs` + este documento.

## Delivery

Estrategia: `ask-on-risk`. Forecast: < 400 líneas → sin split. El merge a `main`
lo aprueba el usuario.

## Verification evidence

- `cargo build` + `cargo test -- --test-threads=1` verdes (3 tests) bajo
  `xvfb-run -a`; sin display el test integrador se omite con aviso.
- Tests nuevos en `src/window.rs`:
  - `results_overlay_visibility_rule` — regla pura: solo vertical muestra el
    overlay, con foco o query no-vacía; sin foco y query vacía no.
  - `responsive_portrait_layout_and_overlay` — via seams `apply_layout` y
    `update_results_visibility` expuestos en `UiHandles`: vertical rota el Paned
    y oculta la lista; con query real "proy" el overlay queda visible; volver a
    horizontal restaura el split clásico.
- Smoke xvfb + i3 + xdotool (screenshots `/tmp/nv-shots2`): resize 500x800
  conmuta el layout (A→B: ~49.7k px); Ctrl+K abre el overlay / Esc lo cierra
  (D→E y E→F: ~4.5k px; W→V y V→U: ~3.6k px) — el overlay PINTA sobre el
  editor. El typing headless no aterriza en el buscador (sin foco X real);
  `search-changed` en GTK4 solo emite ante cambio interactivo del usuario
  (set_text programático no lo emite), cubierto por la regla unitaria.
- Refactor de verificación: `build_ui` devuelve `UiHandles` (widgets + seams) y
  `main.rs` descarta el handle en el closure de `connect_activate`.
- Commit: `ba34aed` feat(ui): responsive portrait search overlay + tests.