package ar.com.nvgtk

import androidx.activity.compose.BackHandler
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.ime
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.input.rememberTextFieldState
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.TextFieldValue
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.unit.LayoutDirection
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.nv_core.NoteSnapshot
import uniffi.nv_core.NvStorage

/**
 * Content editor. The title is editable in place (tap it): confirming calls
 * the core rename API, which may disambiguate on collisions. Content saves
 * only when changed, on back navigation (button or system gesture).
 */
@OptIn(ExperimentalMaterial3Api::class, ExperimentalComposeUiApi::class)
@Composable
fun NoteEditorScreen(
    note: NoteSnapshot,
    storage: NvStorage,
    windowSizeClass: WindowSizeClass,
    autoFocusKeyboard: Boolean = false,
    onDone: () -> Unit,
    onRenamed: (NoteSnapshot) -> Unit
) {
    val scope = rememberCoroutineScope()
    val textFieldState = rememberTextFieldState(initialText = note.content)
    val density = LocalDensity.current
    val imeBottom = WindowInsets.ime.getBottom(density)
    var error by remember { mutableStateOf<String?>(null) }
    var saving by remember { mutableStateOf(false) }
    var editingTitle by remember { mutableStateOf(false) }
    var titleText by remember(note.id) { mutableStateOf(note.title) }
    val focusRequester = remember { FocusRequester() }
    val keyboard = LocalSoftwareKeyboardController.current

    // Auto-open the keyboard only when a brand-new note was just created, so
    // editing can start immediately. Already-existing notes keep their default
    // focus behavior (no IME pop-up on open).
    LaunchedEffect(autoFocusKeyboard) {
        if (autoFocusKeyboard) {
            focusRequester.requestFocus()
            keyboard?.show()
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

    BackHandler {
        if (editingTitle) {
            editingTitle = false
            titleText = note.title
        } else {
            saveIfChanged(onDone)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    if (editingTitle) {
                        TextField(
                            value = titleText,
                            onValueChange = { titleText = it },
                            singleLine = true,
                            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                            keyboardActions = KeyboardActions(onDone = { doRename() })
                        )
                    } else {
                        Text(
                            note.title,
                            modifier = Modifier.clickable { editingTitle = true }
                        )
                    }
                },
                navigationIcon = {
                    IconButton(
                        onClick = { saveIfChanged(onDone) },
                        enabled = !saving
                    ) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = "Volver")
                    }
                },
                actions = {
                    IconButton(onClick = { deleteAndClose() }) {
                        Icon(Icons.Filled.Delete, contentDescription = "Borrar")
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
                OutlinedTextField(
                    state = textFieldState,
                    modifier = Modifier
                        .fillMaxSize()
                        .focusRequester(focusRequester),
                    placeholder = { Text("Escribí tu nota…") }
                )
            }
        }
    }
}
