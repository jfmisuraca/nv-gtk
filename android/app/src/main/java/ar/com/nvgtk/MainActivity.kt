package ar.com.nvgtk

import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.nv_core.NoteSnapshot
import uniffi.nv_core.NvStorage
import java.io.File

class MainActivity : ComponentActivity() {

    private lateinit var storage: NvStorage

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // T2: draw edge-to-edge; Scaffolds consume system bars via their default
        // contentWindowInsets (systemBarsForVisualComponents).
        enableEdgeToEdge()
        val notesDir = File(filesDir, "notes").apply { mkdirs() }
        storage = NvStorage.open(notesDir.absolutePath)
        setContent {
            val darkTheme = isSystemInDarkTheme()
            // Dynamic color (wallpaper palette) on Android 12+, Catppuccin fallback below.
            val colorScheme = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                val context = LocalContext.current
                if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
            } else if (darkTheme) {
                CatppuccinDarkColors
            } else {
                CatppuccinLightColors
            }
            MaterialTheme(colorScheme = colorScheme) {
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
    // T2: width size class, computed once per configuration change (recomposes
    // on rotation/split-screen) and passed down for adaptive layout decisions.
    val windowSizeClass = WindowSizeClass.fromWidth(LocalConfiguration.current.screenWidthDp)
    val scope = rememberCoroutineScope()
    var notes by remember { mutableStateOf<List<NoteSnapshot>>(emptyList()) }
    var trash by remember { mutableStateOf<List<NoteSnapshot>>(emptyList()) }
    var selectedId by remember { mutableStateOf<String?>(null) }
    var showTrash by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var query by remember { mutableStateOf("") }
    var results by remember { mutableStateOf<List<NoteSnapshot>?>(null) }

    fun refresh() {
        scope.launch {
            val result = withContext(Dispatchers.IO) {
                runCatching {
                    val all = storage.listNotes()
                    val trashed = storage.listTrash()
                    val filtered = if (query.isBlank()) null else storage.searchNotes(query)
                    Triple(all, trashed, filtered)
                }
            }
            result
                .onFailure { error = it.message }
                .onSuccess { (freshNotes, freshTrash, freshResults) ->
                    notes = freshNotes
                    trash = freshTrash
                    results = freshResults
                    error = null
                }
        }
    }

    LaunchedEffect(storage) { refresh() }

    // Ranked search with debounce: a new keystroke cancels the previous pass.
    LaunchedEffect(query) {
        if (query.isBlank()) {
            results = null
        } else {
            delay(300)
            results = withContext(Dispatchers.IO) {
                runCatching { storage.searchNotes(query) }
                    .onFailure { error = it.message }
                    .getOrDefault(emptyList())
            }
        }
    }

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
            windowSizeClass = windowSizeClass,
            onBack = { showTrash = false },
            onRestore = { id -> ioOp { storage.restoreNote(id) } },
            onPurge = { id -> ioOp { storage.purgeNote(id) } },
            onEmpty = { ioOp { storage.emptyTrash() } }
        )
        selected == null -> NotesListScreen(
            notes = results ?: notes,
            filtering = results != null,
            query = query,
            error = error,
            windowSizeClass = windowSizeClass,
            onOpen = { selectedId = it },
            onQueryChange = { query = it },
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
            windowSizeClass = windowSizeClass,
            onDone = { selectedId = null; refresh() },
            onRenamed = { renamed -> selectedId = renamed.id; refresh() }
        )
    }
}

/**
 * T2: window width size class with the Material 3 cutoffs
 * (compact < 600dp, medium < 840dp, expanded >= 840dp).
 * Self-implemented: the BOM's material3-adaptive 1.2.0 no longer ships
 * `androidx.compose.material3.adaptive.WindowSizeClass` (the API moved to
 * androidx.window DpSizeClass buckets), so we keep it dependency-free.
 */
enum class WindowSizeClass {
    Compact, Medium, Expanded;

    companion object {
        fun fromWidth(widthDp: Int): WindowSizeClass = when {
            widthDp < 600 -> Compact
            widthDp < 840 -> Medium
            else -> Expanded
        }
    }
}

/** T2: readable content width cap on expanded windows; no cap elsewhere. */
internal val WindowSizeClass.contentMaxWidth: Dp
    get() = if (this == WindowSizeClass.Expanded) 720.dp else Dp.Unspecified
