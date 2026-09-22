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
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Clear
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.ElevatedCard
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.unit.dp


import uniffi.nv_core.NoteSnapshot

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun NotesListScreen(
    notes: List<NoteSnapshot>,
    filtering: Boolean,
    query: String,
    error: String?,
    onOpen: (String) -> Unit,
    onQueryChange: (String) -> Unit,
    onTrash: () -> Unit,
    onCreate: () -> Unit,
    windowSizeClass: WindowSizeClass
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Notas") },
                actions = {
                    IconButton(onClick = onTrash) {
                        Icon(Icons.Filled.Delete, contentDescription = "Papelera")
                    }
                }
            )
        },
        floatingActionButton = {
            // FAB bounces in on first show (spring is StiffnessMediumLow →
            // noticeable but not janky; MediumBouncy gives the playful overshoot).
            val fabEntrance = remember { Animatable(0f) }
            LaunchedEffect(Unit) {
                fabEntrance.animateTo(
                    1f,
                    animationSpec = spring(
                        dampingRatio = Spring.DampingRatioMediumBouncy,
                        stiffness = Spring.StiffnessMediumLow
                    )
                )
            }
            FloatingActionButton(
                onClick = onCreate,
                modifier = Modifier.graphicsLayer {
                    scaleX = fabEntrance.value
                    scaleY = fabEntrance.value
                }
            ) {
                Icon(Icons.Filled.Add, contentDescription = "Nueva nota")
            }
        }
    ) { padding ->
        // T2: cap content width on expanded windows (large screens read a
        // centered 720dp column); no-op on compact/medium. Scaffold's default
        // contentWindowInsets keeps content clear of the system bars.
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
                OutlinedTextField(
                value = query,
                onValueChange = onQueryChange,
                modifier = Modifier.fillMaxWidth().padding(8.dp),
                placeholder = { Text("Buscar…") },
                leadingIcon = { Icon(Icons.Filled.Search, contentDescription = null) },
                trailingIcon = {
                    if (query.isNotEmpty()) {
                        IconButton(onClick = { onQueryChange("") }) {
                            Icon(Icons.Filled.Clear, contentDescription = "Limpiar")
                        }
                    }
                },
                singleLine = true,
                shape = AppShapes.extraLarge
            )
            if (filtering) {
                Text(
                    "${notes.size} resultados",
                    style = LabelMediumEmphasized,
                    modifier = Modifier
                        .padding(horizontal = 12.dp)
                        .animateContentSize()
                )
            }
            if (error != null) {
                Text(
                    error,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(8.dp)
                )
            }
            AnimatedContent(
                targetState = notes.isEmpty(),
                transitionSpec = {
                    // Empty↔list dissolve so the "Sin notas todavía" ghost
                    // never *pops* away from the list; scale crossfade is the
                    // M3 recommened treatment (fade is fine, scale adds depth).
                    (fadeIn() + scaleIn(initialScale = 0.96f)) togetherWith
                        (fadeOut() + scaleOut(targetScale = 0.96f))
                },
                label = "notas-vacio-o-lista"
            ) { empty ->
                if (empty) {
                    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                        Text(
                            "Sin notas todavía",
                            style = BodyLargeEmphasized
                        )
                    }
                } else {
                    LazyColumn(Modifier.fillMaxSize().padding(8.dp)) {
                        items(notes, key = { it.id }) { note ->
                            ElevatedCard(
                                Modifier
                                    .animateItem()
                                    .fillMaxWidth()
                                    .padding(vertical = 4.dp)
                                    .clickable { onOpen(note.id) },
                                shape = AppShapes.largeIncreased
                            ) {
                                Column(Modifier.padding(12.dp)) {
                                    Text(note.title, style = MaterialTheme.typography.titleMedium)
                                    if (note.content.isNotBlank()) {
                                        Spacer(Modifier.height(4.dp))
                                        Text(
                                            note.content.take(140),
                                            style = MaterialTheme.typography.bodyMedium,
                                            maxLines = 3
                                        )
                                    }
                                    if (note.tags.isNotEmpty()) {
                                        Spacer(Modifier.height(4.dp))
                                        Text(
                                            note.tags.joinToString(" ") { "#$it" },
                                            style = MaterialTheme.typography.labelSmall
                                        )
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
