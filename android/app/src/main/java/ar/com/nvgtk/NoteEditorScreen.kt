package ar.com.nvgtk

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.input.TextFieldState
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.text.TextRange
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.conflate
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.nv_core.NoteSnapshot
import uniffi.nv_core.NvStorage
import uniffi.nv_core.WikiCandidate
import uniffi.nv_core.WikiLink
import uniffi.nv_core.linkAtCursor
import uniffi.nv_core.openWikiQuery

/**
 * Minimum touch-target height for the wiki UI (Material guidance: 48dp).
 * Shared by the follow-chip and the suggestion rows so a finger-width change
 * in one place cannot shrink the other below the minimum.
 */
internal val WikiTouchTargetMinHeight = 48.dp

/**
 * Content editor. The title is editable in place (tap it): confirming calls
 * the core rename API, which may disambiguate on collisions. Content
 * autosaves write-through (no debounce tail to lose on a fast system back);
 * the back arrow also flushes any unsaved change before popping.
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalComposeUiApi::class)
@Composable
fun NoteEditorScreen(
    note: NoteSnapshot,
    storage: NvStorage,
    windowSizeClass: WindowSizeClass,
    autoFocusKeyboard: Boolean = false,
    onDone: () -> Unit,
    onRenamed: (NoteSnapshot) -> Unit,
    onFollowLink: (String) -> Unit
) {
    val scope = rememberCoroutineScope()
    val editorFieldLabel = stringResource(R.string.editor_field_label)
    val titleFieldLabel = stringResource(R.string.title_field_label)
    val emptyTitle = stringResource(R.string.note_empty_title)
    val noMatches = stringResource(R.string.wiki_no_matches)
    val suggestionsLabel = stringResource(R.string.wiki_suggestions_label)
    // Keyed on the note id so following a wiki-link into another note (same
    // route, new note) resets the field instead of keeping stale text.
    val textFieldState = remember(note.id) { TextFieldState(note.content) }
    val density = LocalDensity.current
    val imeBottom = WindowInsets.ime.getBottom(density)
    var error by remember { mutableStateOf<String?>(null) }
    var saving by remember { mutableStateOf(false) }
    var editingTitle by remember { mutableStateOf(false) }
    var titleText by remember(note.id) { mutableStateOf(note.title) }
    var savedText by remember(note.id) { mutableStateOf(note.content) }
    val focusRequester = remember { FocusRequester() }
    val keyboard = LocalSoftwareKeyboardController.current

    // Wiki-links (parity slice 3, rules in nv-core): pending `[[query`
    // autocompletion plus a follow chip on closed `[[link]]` under cursor.
    // `openWikiQuery` runs on one line (main-thread cheap); ranking and the
    // cursor lookup run on IO because they scan the whole note/storage.
    val fullText = textFieldState.text.toString()
    val cursorPos = textFieldState.selection.start.coerceIn(0, fullText.length)
    val openQuery = remember(fullText, cursorPos) {
        runCatching { openWikiQuery(lineBeforeCursor(fullText, cursorPos)) }.getOrNull()
    }
    var activeLink by remember { mutableStateOf<WikiLink?>(null) }
    LaunchedEffect(fullText, cursorPos) {
        // Kotlin offsets are UTF-16; the core counts Unicode scalars, so the
        // cursor is converted (never panics, degrades to no-chip on error).
        val cursorChars = runCatching {
            fullText.codePointCount(0, cursorPos).toULong()
        }.getOrNull()
        activeLink = if (cursorChars == null) {
            null
        } else {
            withContext(Dispatchers.IO) {
                runCatching { linkAtCursor(fullText, cursorChars) }.getOrNull()
            }
        }
    }
    var suggestions by remember { mutableStateOf<List<WikiCandidate>>(emptyList()) }
    // T8: a cursor inside a closed `[[link]]` still yields a partial open
    // query (the core only sees the line before the cursor), so the fetch is
    // also suppressed while a closed link is under the cursor — follow wins.
    LaunchedEffect(openQuery, activeLink) {
        val q = openQuery
        suggestions = if (q == null || activeLink != null) {
            emptyList()
        } else {
            withContext(Dispatchers.IO) {
                runCatching { storage.wikiSuggest(q) }.getOrDefault(emptyList())
            }
        }
    }

    // Insert the chosen candidate over the pending `[[query` (desktop parity:
    // brackets are kept, the display title is closed with `]]`).
    fun acceptCandidate(candidate: WikiCandidate) {
        val q = openQuery ?: return
        val cur = textFieldState.selection.start.coerceIn(0, textFieldState.text.length)
        val start = pendingWikiStart(cur, q).coerceAtMost(cur)
        textFieldState.edit {
            replace(start, cur, candidate.displayTitle + "]]")
            selection = TextRange(start + candidate.displayTitle.length + 2)
        }
    }

    // Auto-open the keyboard only when a brand-new note was just created, so
    // editing can start immediately. Already-existing notes keep their default
    // focus behavior (no IME pop-up on open).
    LaunchedEffect(autoFocusKeyboard) {
        if (autoFocusKeyboard) {
            focusRequester.requestFocus()
            keyboard?.show()
        }
    }

    // Autosave: persist the latest text write-through (no artificial debounce)
    // so a SYSTEM back gesture (which bypasses onDone) never loses edits. A
    // fresh emission while one write is in-flight is coalesced by `conflate`,
    // not dropped: the current write finishes and the latest text is written
    // next. This keeps the loss window to a single in-flight write (ms), not a
    // debounce tail that a fast 3-button back would cancel.
    LaunchedEffect(textFieldState) {
        snapshotFlow { textFieldState.text.toString() }
            .conflate()
            .collect { current ->
                if (current != savedText && !saving) {
                    saving = true
                    withContext(Dispatchers.IO) {
                        runCatching { storage.saveNote(note.id, current) }
                    }
                        .onFailure { error = it.message }
                        .onSuccess { savedText = current }
                    saving = false
                }
            }
    }

    fun saveIfChanged(next: () -> Unit) {
        val current = textFieldState.text.toString()
        if (current == note.content) {
            next()
            return
        }
        saving = true
        scope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching { storage.saveNote(note.id, current) }
            }
            saving = false
            result.onFailure { error = it.message }.onSuccess { next() }
        }
    }

    fun deleteAndClose() {
        scope.launch {
            withContext(Dispatchers.IO) {
                runCatching { storage.deleteNote(note.id) }
            }
                .onFailure { error = it.message }
                .onSuccess { onDone() }
        }
    }

    fun doRename() {
        val want = titleText.trim()
        if (want.isEmpty() || want == note.title) {
            editingTitle = false
            titleText = note.title
            return
        }
        scope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching { storage.renameNote(note.id, want) }
            }
            result.onFailure { error = it.message }.onSuccess {
                editingTitle = false
                onRenamed(it)
            }
        }
    }

    // Title editing is IN-SCREEN state, not navigation: back only closes title
    // editing while active. Real navigation back reaches the NavController
    // untouched so the predictive-back animation can play.
    BackHandler(enabled = editingTitle) {
        editingTitle = false
        titleText = note.title
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    if (editingTitle) {
                        TextField(
                            value = titleText,
                            onValueChange = { titleText = it },
                            modifier = Modifier.semantics { contentDescription = titleFieldLabel },
                            singleLine = true,
                            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                            keyboardActions = KeyboardActions(onDone = { doRename() })
                        )
                    } else {
                        // The title is an in-place edit affordance: expose it as
                        // a button (role + 48dp minimum touch target, centered so
                        // the glyphs never move).
                        // Derived title (desktop parity) follows the LIVE text so
                        // the bar reflects what is typed; rename still renames
                        // the FILE (doRename/titleText untouched).
                        Box(
                            Modifier
                                .heightIn(min = 48.dp)
                                .clickable(role = Role.Button) { editingTitle = true },
                            contentAlignment = Alignment.Center
                        ) {
                            Text(
                                displayTitle(textFieldState.text.toString(), emptyTitle),
                                style = HeadlineMediumEmphasized
                            )
                        }
                    }
                },
                navigationIcon = {
                    IconButton(
                        onClick = { saveIfChanged(onDone) },
                        enabled = !saving
                    ) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                },
                actions = {
                    IconButton(onClick = { deleteAndClose() }) {
                        Icon(Icons.Filled.Delete, contentDescription = stringResource(R.string.delete_note))
                    }
                }
            )
        }
    ) { padding ->
        // T2-fix: Scaffold innerPadding already includes the nav-bar bottom inset.
        // Stacking `imePadding()` on top of it would float the field above the
        // keyboard, so the bottom inset is applied conditionally instead.
        Box(
            Modifier
                .padding(
                    start = padding.calculateLeftPadding(LayoutDirection.Ltr),
                    top = padding.calculateTopPadding(),
                    end = padding.calculateRightPadding(LayoutDirection.Ltr)
                )
                .fillMaxSize(),
            contentAlignment = Alignment.TopCenter
        ) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .widthIn(max = windowSizeClass.contentMaxWidth)
                    .fillMaxHeight()
                    .animateContentSize()
                    .then(if (imeBottom > 0) Modifier.imePadding() else Modifier.navigationBarsPadding())
                    .padding(8.dp)
            ) {
                AnimatedVisibility(visible = error != null) {
                    Text(
                        error ?: "",
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.padding(bottom = 8.dp)
                    )
                }
                // Follow-chip: the cursor sits on a closed `[[link]]`.
                // Opens the existing note or creates it (desktop flow).
                // T6: full-width 48dp touch target (Material minimum) so the
                // chip is comfortable to tap; text stays start-aligned.
                AnimatedVisibility(visible = activeLink != null) {
                    val link = activeLink
                    if (link != null) {
                        Box(
                            modifier = Modifier
                                .fillMaxWidth()
                                .heightIn(min = WikiTouchTargetMinHeight)
                                .clickable(role = Role.Button) { onFollowLink(link.target) }
                                .padding(bottom = 8.dp),
                            contentAlignment = Alignment.CenterStart
                        ) {
                            Text(
                                text = stringResource(R.string.wiki_open_link, link.target),
                                color = MaterialTheme.colorScheme.primary
                            )
                        }
                    }
                }
                // Autocomplete panel for a pending `[[query` (core-ranked).
                // T8: hidden while a closed `[[link]]` is under the cursor —
                // the follow-chip above owns that state (desktop parity:
                // closed link navigates, only an open trigger autocompletes).
                AnimatedVisibility(visible = shouldShowWikiAutocomplete(openQuery, activeLink != null)) {
                    ElevatedCard(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(bottom = 8.dp)
                            .semantics { contentDescription = suggestionsLabel }
                    ) {
                        if (suggestions.isEmpty()) {
                            Text(
                                noMatches,
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                                modifier = Modifier.padding(12.dp)
                            )
                        } else {
                            LazyColumn(Modifier.heightIn(max = 240.dp)) {
                                items(suggestions, key = { it.id }) { candidate ->
                                    // T6: 48dp minimum touch height (Material),
                                    // content vertically centered so short
                                    // single-line titles still fill the row.
                                    Row(
                                        modifier = Modifier
                                            .fillMaxWidth()
                                            .heightIn(min = WikiTouchTargetMinHeight)
                                            .clickable { acceptCandidate(candidate) }
                                            .padding(
                                                horizontal = 12.dp,
                                                vertical = 8.dp
                                            ),
                                        verticalAlignment = Alignment.CenterVertically
                                    ) {
                                        Text(
                                            candidate.displayTitle,
                                            style = MaterialTheme.typography.titleSmall,
                                            modifier = Modifier.weight(1f)
                                        )
                                        if (candidate.tags.isNotEmpty()) {
                                            Text(
                                                candidate.tags.joinToString(" ") { "#$it" },
                                                style = MaterialTheme.typography.labelSmall,
                                                color = MaterialTheme.colorScheme.onSurfaceVariant
                                            )
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                OutlinedTextField(
                    state = textFieldState,
                    modifier = Modifier
                        .fillMaxSize()
                        .focusRequester(focusRequester)
                        .semantics { contentDescription = editorFieldLabel },
                    placeholder = { Text(stringResource(R.string.editor_placeholder)) }
                )
            }
        }
    }
}
