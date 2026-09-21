package ar.com.nvgtk

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.nv_core.NoteSnapshot
import uniffi.nv_core.NvStorage
import java.io.File

class MainActivity : ComponentActivity() {

    private lateinit var storage: NvStorage

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val notesDir = File(filesDir, "notes").apply { mkdirs() }
        storage = NvStorage.open(notesDir.absolutePath)
        setContent {
            MaterialTheme {
                Surface(Modifier.fillMaxSize()) {
                    NvApp(storage)
                }
            }
        }
    }

    override fun onDestroy() {
        if (::storage.isInitialized) {
            storage.destroy()
        }
        super.onDestroy()
    }
}

/**
 * Root state: note list + state-based navigation (no nav library for two
 * screens). Storage I/O always runs on [Dispatchers.IO].
 */
@Composable
private fun NvApp(storage: NvStorage) {
    val scope = rememberCoroutineScope()
    var notes by remember { mutableStateOf<List<NoteSnapshot>>(emptyList()) }
    var trash by remember { mutableStateOf<List<NoteSnapshot>>(emptyList()) }
    var selectedId by remember { mutableStateOf<String?>(null) }
    var showTrash by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    fun refresh() {
        scope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching {
                    storage.listNotes() to storage.listTrash()
                }
            }
            result
                .onFailure { error = it.message }
                .onSuccess { (freshNotes, freshTrash) ->
                    notes = freshNotes
                    trash = freshTrash
                    error = null
                }
        }
    }

    LaunchedEffect(storage) { refresh() }

    fun ioOp(op: suspend () -> Unit) {
        scope.launch {
            withContext(Dispatchers.IO) { runCatching { op() } }
                .onFailure { error = it.message }
                .onSuccess { refresh() }
        }
    }

    val selected = notes.firstOrNull { it.id == selectedId }
    when {
        showTrash -> TrashScreen(
            trash = trash,
            error = error,
            onBack = { showTrash = false },
            onRestore = { id -> ioOp { storage.restoreNote(id) } },
            onPurge = { id -> ioOp { storage.purgeNote(id) } },
            onEmpty = { ioOp { storage.emptyTrash() } }
        )
        selected == null -> NotesListScreen(
            notes = notes,
            error = error,
            onOpen = { selectedId = it },
            onTrash = { showTrash = true },
            onCreate = {
                scope.launch {
                    withContext(Dispatchers.IO) {
                        runCatching { storage.createNote("Nota nueva") }
                    }
                        .onFailure { error = it.message }
                        .onSuccess { created ->
                            notes = withContext(Dispatchers.IO) { storage.listNotes() }
                            error = null
                            selectedId = created.id
                        }
                }
            }
        )
        else -> NoteEditorScreen(
            note = selected,
            storage = storage,
            onDone = { selectedId = null; refresh() }
        )
    }
}
