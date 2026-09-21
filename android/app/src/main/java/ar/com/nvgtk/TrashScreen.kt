package ar.com.nvgtk

import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
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
    onBack: () -> Unit,
    onRestore: (String) -> Unit,
    onPurge: (String) -> Unit,
    onEmpty: () -> Unit
) {
    var pendingPurge by remember { mutableStateOf<NoteSnapshot?>(null) }
    var confirmEmpty by remember { mutableStateOf(false) }

    BackHandler(onBack = onBack)

    if (pendingPurge != null) {
        AlertDialog(
            onDismissRequest = { pendingPurge = null },
            title = { Text("Eliminar definitivamente") },
            text = { Text("¿Borrar \"${pendingPurge?.title}\" para siempre?") },
            confirmButton = {
                TextButton(onClick = {
                    pendingPurge?.let { onPurge(it.id) }
                    pendingPurge = null
                }) {
                    Text("Eliminar")
                }
            },
            dismissButton = {
                TextButton(onClick = { pendingPurge = null }) {
                    Text("Cancelar")
                }
            }
        )
    }

    if (confirmEmpty) {
        AlertDialog(
            onDismissRequest = { confirmEmpty = false },
            title = { Text("Vaciar papelera") },
            text = { Text("¿Eliminar para siempre las ${trash.size} notas?") },
            confirmButton = {
                TextButton(onClick = { confirmEmpty = false; onEmpty() }) {
                    Text("Vaciar")
                }
            },
            dismissButton = {
                TextButton(onClick = { confirmEmpty = false }) {
                    Text("Cancelar")
                }
            }
        )
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Papelera") },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = "Volver")
                    }
                },
                actions = {
                    TextButton(
                        onClick = { confirmEmpty = true },
                        enabled = trash.isNotEmpty()
                    ) {
                        Text("Vaciar")
                    }
                }
            )
        }
    ) { padding ->
        Column(Modifier.padding(padding).fillMaxSize()) {
            if (error != null) {
                Text(
                    error,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(8.dp)
                )
            }
            if (trash.isEmpty()) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                    Text("Papelera vacía")
                }
            } else {
                LazyColumn(Modifier.fillMaxSize().padding(8.dp)) {
                    items(trash, key = { it.id }) { note ->
                        ElevatedCard(
                            Modifier
                                .fillMaxWidth()
                                .padding(vertical = 4.dp)
                        ) {
                            Row(
                                Modifier.padding(12.dp),
                                verticalAlignment = Alignment.CenterVertically
                            ) {
                                Text(
                                    note.title,
                                    style = MaterialTheme.typography.titleMedium,
                                    modifier = Modifier.weight(1f)
                                )
                                TextButton(onClick = { onRestore(note.id) }) {
                                    Text("Restaurar")
                                }
                                IconButton(onClick = { pendingPurge = note }) {
                                    Icon(Icons.Filled.Delete, contentDescription = "Eliminar")
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
