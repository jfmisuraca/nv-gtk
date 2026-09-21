# Storage contract (nv-gtk ↔ Android parity)

Source of truth is the desktop code (`nv-core/src/storage.rs`, `nv-core/src/note.rs`,
`nv-core/src/config.rs`, `nv-core/src/util.rs`). This spec freezes its *observable*
behavior so a future Android implementation can match it exactly.
Nothing here invents behavior: every rule cites the code that implements it.

## 1. Notes directory

- The notes directory comes from `Config::notes_dir` (`nv-core/src/config.rs`).
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

- `StorageManager::reload` (`nv-core/src/storage.rs`) iterates the notes directory
  (non-recursive, files only) and loads **only** files whose extension is
  exactly `md`, `txt`, or `markdown`.
- The comparison is case-sensitive (`ext == "md"`, …): `NOTE.MD` is ignored.
- Files with any other extension — or no extension — are silently skipped.
- A file that fails to read is skipped as well (`if let Ok(note)`).

Observable: dropping `a.md`, `b.txt`, `c.markdown`, `d.rs`, `e.MD`, and
`Makefile` into the directory loads exactly `a`, `b`, `c`.

## 3. Identity: id = filename stem

- `Note::from_file` (`nv-core/src/note.rs`) sets `id` to the file's stem
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
  falls back to mtime (`nv-core/src/note.rs`, `from_file`).
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

- Creation (`StorageManager::create_note`, `nv-core/src/storage.rs`): if the
  requested title already exists — checked case-insensitively against loaded
  titles **or** as `<title>.<default_extension>` on disk
  (`note_title_exists`) — the note is instead created with
  `timestamp_title_with_seconds()` (`%Y%m%d-%H%M%S` from `nv-core/src/util.rs`),
  appending `-<counter>` while that also exists. The original file is never
  overwritten.
- Save (`StorageManager::save_note`): writes the file **only** when content
  actually changed; then re-sorts the list (rule 7). Returns `Ok(true)` on
  success (including unchanged content), `Ok(false)` for unknown ids, and
  `Err` when the file cannot be written — in that case the in-memory content
  is still updated (the caller's edits are preserved) but persistence failed.
- Delete (`StorageManager::delete_note`): `Ok(true)` removes the file and the
  entry; `Ok(false)` for unknown ids; `Err` when removal fails — the entry is
  then kept in memory, matching what is still on disk.
- Creation (`StorageManager::create_note`): `Err` when the note file cannot be
  written, inserting nothing — no in-memory ghosts for unpersisted notes.
- FFI (`nv-core/src/ffi.rs`): persistence failures surface as `NvError::Io`,
  unknown ids as `NvError::NotFound` (`get`/`save`) or `Ok(false)` (`delete`).
- `Note::save`: `fs::write` overwrites the file, refreshes `modified_at` to
  now, and re-derives tags from the new content (rule 5). The note keeps its
  `id`/`filepath`.

Observable: creating `"20240919-1530"` three times yields three distinct
files/ids; saving new content overwrites the same file, updates its mtime
and tags, and re-sorts the list.

## 9. App trash (not the OS trash)

- Deleted notes are moved to `notes_dir/.trash/` (same-filesystem `rename`,
  so dates and tags survive), never unlinked
  (`StorageManager::delete_note`, `nv-core/src/storage.rs`).
- The trash directory is hidden and never loaded: `reload` only reads files
  directly inside the notes dir (`is_file` skips the directory).
- `trash_notes` lists trashed notes newest-first (same format filter as rule 2).
- `restore_note` moves the file back; when its name was retaken meanwhile,
  the restored note gets a timestamp-disambiguated id instead of overwriting.
- `purge_note` unlinks one trashed note; `empty_trash` unlinks all of them
  and returns the purged count.
- A move that would overwrite an existing trash file is disambiguated with a
  `-trash-<n>` suffix instead.

Observable: deleting `a.md` removes it from the list but keeps
`.trash/a.md`; restoring brings it back; purging or emptying deletes for good.

## 10. Rename (new filename stem)

- `StorageManager::rename_note` (`nv-core/src/storage.rs`) moves the file to
  `<new-title>.<default_extension>` and updates `id`/`title` together.
- Blank titles become `"Untitled"`, `/` becomes `-` (same as creation);
  taken names disambiguate timestamp-style like creation, excluding the note
  itself (so case-only renames work); the original file is never overwritten.
- Content, tags and dates are preserved: the move keeps mtime, so a rename
  does not re-sort the list. Unknown ids return `None` (FFI: `NotFound`).

Observable: renaming `a` to `b` leaves `b.md` with `a`'s content, tags and
mtime; renaming onto a taken name yields a distinct id.
