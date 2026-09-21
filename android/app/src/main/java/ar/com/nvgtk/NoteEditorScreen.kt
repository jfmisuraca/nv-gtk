package ar.com.nvgtk

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.nv_core.NoteSnapshot
import uniffi.nv_core.NvStorage

/**
 * Content editor. The title is the note id (filename stem, contract rule 4)
 * and there is no rename API, so it renders read-only. Content saves only
 * when changed, on back navigation (button or system gesture).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NoteEditorScreen(
    note: NoteSnapshot,
    storage: NvStorage,
    onDone: () -> Unit
) {
    val scope = rememberCoroutineScope()
    var text by remember(note.id) { mutableStateOf(note.content) }
    var error by remember { mutableStateOf<String?>(null) }
    var saving by remember { mutableStateOf(false) }
    var confirmDelete by remember { mutableStateOf(false) }

    fun saveIfChanged(next: () -> Unit) {
        if (text == note.content) {
            next()
            return
        }
        saving = true
        scope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching { storage.saveNote(note.id, text) }
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

    BackHandler { saveIfChanged(onDone) }

    if (confirmDelete) {
        AlertDialog(
            onDismissRequest = { confirmDelete = false },
            title = { Text("Borrar nota") },
            text = { Text("¿Borrar \"${note.title}\" para siempre?") },
            confirmButton = {
                TextButton(onClick = { confirmDelete = false; deleteAndClose() }) {
                    Text("Borrar")
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmDelete = false }) {
                    Text("Cancelar")
                }
            }
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(note.title) },
                navigationIcon = {
                    IconButton(
                        onClick = { saveIfChanged(onDone) },
                        enabled = !saving
                    ) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = "Volver")
                    }
                },
                actions = {
                    IconButton(onClick = { confirmDelete = true }) {
                        Icon(Icons.Filled.Delete, contentDescription = "Borrar")
                    }
                }
            )
        }
    ) { padding ->
        Column(Modifier.padding(padding).fillMaxSize().padding(8.dp)) {
            if (error != null) {
                Text(
                    error ?: "",
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(bottom = 8.dp)
                )
            }
            OutlinedTextField(
                value = text,
                onValueChange = { text = it },
                modifier = Modifier.fillMaxSize(),
                placeholder = { Text("Escribí tu nota…") }
            )
        }
    }
}
