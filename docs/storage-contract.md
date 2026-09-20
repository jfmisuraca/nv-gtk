# Storage contract (nv-gtk ↔ Android parity)

Source of truth is the desktop code (`src/storage.rs`, `src/note.rs`,
`src/config.rs`, `src/app_state.rs`). This spec freezes its *observable*
behavior so a future Android implementation can match it exactly.
Nothing here invents behavior: every rule cites the code that implements it.

## 1. Notes directory

- The notes directory comes from `Config::notes_dir` (`src/config.rs`).
- Default is `$HOME/Notes` with `default_extension = "md"` (`Config::default`).
- The config file is `<config_dir>/nv-gtk/config.json`
  (`Config::config_path`); `Config::load` reads it when present and valid,
  otherwise falls back to the default and saves it.
- `Config::load` and `Config::save` both ensure the notes directory exists
  (`fs::create_dir_all`).
- `StorageManager::new` adopts `config.notes_dir` /
  `config.default_extension` and immediately loads (`reload`).

Observable: point the config at a directory and its `*.md` files appear as
notes; a missing directory is created, never an error.

## 2. Valid extensions

- `StorageManager::reload` (`src/storage.rs`) iterates the notes directory
  (non-recursive, files only) and loads **only** files whose extension is
  exactly `md`, `txt`, or `markdown`.
- The comparison is case-sensitive (`ext == "md"`, …): `NOTE.MD` is ignored.
- Files with any other extension — or no extension — are silently skipped.
- A file that fails to read is skipped as well (`if let Ok(note)`).

Observable: dropping `a.md`, `b.txt`, `c.markdown`, `d.rs`, `e.MD`, and
`Makefile` into the directory loads exactly `a`, `b`, `c`.

## 3. Identity: id = filename stem

- `Note::from_file` (`src/note.rs`) sets `id` to the file's stem
  (`path.file_stem()`), i.e. the filename without extension.
- `Note::new` sets `id` to the sanitized title (`/` → `-`); the file is
  `<title>.<default_extension>` inside the notes directory.
- `StorageManager::get_note` / `delete_note` / `save_note` look notes up by
  this `id`.

Observable: `my-note.md` ⇔ `id == "my-note"`; renaming the file re-identifies
the note.

## 4. Display title = filename stem (NOT the first content line)

- `Note::from_file` sets `title` to the same filename stem as `id`.
- File content never influences the title: a file whose first non-empty line
  is `# Hello` still has `title == <filename stem>`.
- Fallback when the stem is missing/invalid UTF-8: `"Untitled"`.
- On creation, an empty/blank requested title becomes `"Untitled"`, and `/`
  in titles is replaced with `-` (`Note::new`, `StorageManager::create_note`).

Observable: title and id are equal at load; editing content never renames a
note. (The feature brief's "first non-empty line" wording does **not** match
the code — the code behavior above is authoritative.)

## 5. Raw content + derived tags

- `content` is the file's raw UTF-8 text, byte-for-byte (`fs::read_to_string`;
  no trimming, no front-matter stripping).
- `tags` are derived at load (`from_file`) and re-derived on every `save`
  via `Note::extract_tags`: regex `#([a-zA-Z0-9_-]+)`, then sorted and
  deduplicated.
- Tags are never persisted separately; they are always recomputed from
  content. A note created with `Note::new` starts with empty content and no
  tags.

Observable: content `"see #beta and #alpha #beta"` ⇒
`tags == ["alpha", "beta"]`; `#with-dash_1` is one tag; `#!` / `#` alone match
nothing.

## 6. Dates from filesystem metadata

- `modified_at` = filesystem mtime; if mtime is unavailable, now.
- `created_at` = filesystem birthtime; if unavailable (common on Linux),
  falls back to mtime (`src/note.rs`, `from_file`).
- `Note::new` stamps both with `Local::now()`; `save` refreshes
  `modified_at` to now but leaves `created_at` untouched.

Observable: `modified_at` tracks the file's mtime; on filesystems without
birthtime, `created_at == modified_at`.

## 7. Ordering by modified_at descending

- After every `reload` and every `save_note`, notes are sorted newest-first
  (`b.modified_at.cmp(&a.modified_at)`).
- `create_note` inserts the new note at index 0 (it carries `Local::now()`,
  i.e. the newest timestamp).

Observable: list order is always newest-modified first; saving a note moves
it toward the top.

## 8. Collisions and save semantics

- Creation (`StorageManager::create_note`, `src/storage.rs`): if the
  requested title already exists — checked case-insensitively against loaded
  titles **or** as `<title>.<default_extension>` on disk
  (`note_title_exists`) — the note is instead created with
  `timestamp_title_with_seconds()` (`%Y%m%d-%H%M%S` from `src/app_state.rs`),
  appending `-<counter>` while that also exists. The original file is never
  overwritten.
- Save (`StorageManager::save_note`): writes the file **only** when content
  actually changed; then re-sorts the list (rule 7).
- `Note::save`: `fs::write` overwrites the file, refreshes `modified_at` to
  now, and re-derives tags from the new content (rule 5). The note keeps its
  `id`/`filepath`.

Observable: creating `"20240919-1530"` three times yields three distinct
files/ids; saving new content overwrites the same file, updates its mtime
and tags, and re-sorts the list.
