package ar.com.nvgtk

import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.togetherWith
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import uniffi.nv_core.NoteSnapshot

/**
 * App trash (contract rule 9): restore, purge one (confirmed) or empty all
 * (confirmed). Purging is permanent — here "para siempre" is accurate.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TrashScreen(
    trash: List<NoteSnapshot>,
    error: String?,
    windowSizeClass: WindowSizeClass,
    onBack: () -> Unit,
    onRestore: (String) -> Unit,
    onPurge: (String) -> Unit,
    onEmpty: () -> Unit
) {
    var pendingPurge by remember { mutableStateOf<NoteSnapshot?>(null) }
    var confirmEmpty by remember { mutableStateOf(false) }
    var openedId by remember { mutableStateOf<String?>(null) }
    // Derived titles (desktop parity); the stem stays identity everywhere.
    val emptyTitle = stringResource(R.string.note_empty_title)

    val opened = trash.firstOrNull { it.id == openedId }
    if (opened != null) {
        TrashNoteDetail(
            note = opened,
            windowSizeClass = windowSizeClass,
            onBack = { openedId = null },
            onRestore = { onRestore(opened.id); openedId = null },
            onPurge = { onPurge(opened.id); openedId = null }
        )
        return
    }

    if (pendingPurge != null) {
        AlertDialog(
            onDismissRequest = { pendingPurge = null },
            title = { Text(stringResource(R.string.purge_dialog_title)) },
            text = { Text(stringResource(R.string.confirm_purge, pendingPurge?.let { displayTitle(it.content, emptyTitle) } ?: "")) },
            confirmButton = {
                TextButton(onClick = {
                    pendingPurge?.let { onPurge(it.id) }
                    pendingPurge = null
                }) {
                    Text(stringResource(R.string.delete))
                }
            },
            dismissButton = {
                TextButton(onClick = { pendingPurge = null }) {
                    Text(stringResource(R.string.cancel))
                }
            }
        )
    }

    if (confirmEmpty) {
        AlertDialog(
            onDismissRequest = { confirmEmpty = false },
            title = { Text(stringResource(R.string.empty_trash_dialog_title)) },
            text = { Text(pluralStringResource(R.plurals.confirm_empty_count, trash.size, trash.size)) },
            confirmButton = {
                TextButton(onClick = { confirmEmpty = false; onEmpty() }) {
                    Text(stringResource(R.string.empty_trash_action))
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmEmpty = false }) {
                    Text(stringResource(R.string.cancel))
                }
            }
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.trash_title)) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                },
                actions = {
                    TextButton(
                        onClick = { confirmEmpty = true },
                        enabled = trash.isNotEmpty()
                    ) {
                        Text(stringResource(R.string.empty_trash_action))
                    }
                }
            )
        }
    ) { padding ->
        // T2: same max-width treatment as the list/editor on expanded windows.
        Box(
            Modifier.padding(padding).fillMaxSize(),
            contentAlignment = Alignment.TopCenter
        ) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .widthIn(max = windowSizeClass.contentMaxWidth)
                    .fillMaxHeight()
            ) {
                if (error != null) {
                    Text(
                        error,
                        color = MaterialTheme.colorScheme.error,
                        modifier = Modifier.padding(8.dp)
                    )
                }
                AnimatedContent(
                    targetState = trash.isEmpty(),
                    transitionSpec = {
                        (fadeIn() + scaleIn(initialScale = 0.96f)) togetherWith
                            (fadeOut() + scaleOut(targetScale = 0.96f))
                    },
                    label = "papelera-vacio-o-lista"
                ) { empty ->
                    if (empty) {
                        Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                            Text(
                                stringResource(R.string.empty_trash),
                                style = BodyLargeEmphasized
                            )
                        }
                    } else {
                        LazyColumn(Modifier.fillMaxSize().padding(8.dp)) {
                            items(trash, key = { it.id }) { note ->
                            ElevatedCard(
                                onClick = { openedId = note.id },
                                modifier = Modifier
                                    .animateItem()
                                    .fillMaxWidth()
                                    .padding(vertical = 4.dp),
                                shape = AppShapes.largeIncreased
                            ) {
                                Row(
                                    Modifier.padding(20.dp),
                                    verticalAlignment = Alignment.CenterVertically
                                ) {
                                    Text(
                                        displayTitle(note.content, emptyTitle),
                                        style = MaterialTheme.typography.titleMedium,
                                        modifier = Modifier.weight(1f)
                                    )
                                    TextButton(onClick = { onRestore(note.id) }) {
                                        Text(stringResource(R.string.restore))
                                    }
                                    IconButton(onClick = { pendingPurge = note }) {
                                        Icon(Icons.Filled.Delete, contentDescription = stringResource(R.string.delete))
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
}

/**
 * Read-only preview of a trashed note: full content (selectable) plus
 * restore / purge actions, so the user can decide its fate.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TrashNoteDetail(
    note: NoteSnapshot,
    windowSizeClass: WindowSizeClass,
    onBack: () -> Unit,
    onRestore: () -> Unit,
    onPurge: () -> Unit
) {
    var confirmPurge by remember { mutableStateOf(false) }
    val emptyTitle = stringResource(R.string.note_empty_title)

    BackHandler(onBack = onBack)

    if (confirmPurge) {
        AlertDialog(
            onDismissRequest = { confirmPurge = false },
            title = { Text(stringResource(R.string.purge_dialog_title)) },
            text = { Text(stringResource(R.string.confirm_purge, displayTitle(note.content, emptyTitle))) },
            confirmButton = {
                TextButton(onClick = { confirmPurge = false; onPurge() }) {
                    Text(stringResource(R.string.delete))
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmPurge = false }) {
                    Text(stringResource(R.string.cancel))
                }
            }
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Text(
                        displayTitle(note.content, emptyTitle),
                        style = TitleLargeEmphasized
                    )
                },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                },
                actions = {
                    TextButton(onClick = onRestore) {
                        Text(stringResource(R.string.restore))
                    }
                    IconButton(onClick = { confirmPurge = true }) {
                        Icon(Icons.Filled.Delete, contentDescription = stringResource(R.string.delete))
                    }
                }
            )
        }
    ) { padding ->
        Box(
            Modifier.padding(padding).fillMaxSize(),
            contentAlignment = Alignment.TopCenter
        ) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .widthIn(max = windowSizeClass.contentMaxWidth)
                    .fillMaxHeight()
                    .verticalScroll(rememberScrollState())
                    .padding(12.dp)
            ) {
                SelectionContainer {
                    // Emphasize only the empty placeholder; real content must
                    // stay regular so it never competes with the title.
                    if (note.content.isBlank()) {
                        Text(
                            stringResource(R.string.no_content_placeholder),
                            style = BodyLargeEmphasized
                        )
                    } else {
                        Text(
                            note.content,
                            style = MaterialTheme.typography.bodyLarge
                        )
                    }
                }
                if (note.tags.isNotEmpty()) {
                    Spacer(Modifier.height(12.dp))
                    Text(
                        note.tags.joinToString(" ") { "#$it" },
                        style = MaterialTheme.typography.labelLarge
                    )
                }
            }
        }
    }
}
