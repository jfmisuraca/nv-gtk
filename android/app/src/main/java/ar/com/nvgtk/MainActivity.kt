package ar.com.nvgtk

import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.scaleOut
import androidx.compose.ui.graphics.TransformOrigin
import androidx.navigationevent.NavigationEvent
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Search
import androidx.compose.material3.FloatingActionButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.Surface
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Alignment
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.nv_core.NoteSnapshot
import uniffi.nv_core.NvStorage
import java.io.File

/** T2: window width size class with the Material 3 cutoffs. */
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

class MainActivity : ComponentActivity() {

    private lateinit var storage: NvStorage

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        val notesDir = File(filesDir, "notes").apply { mkdirs() }
        storage = NvStorage.open(notesDir.absolutePath)
        setContent {
            val darkTheme = isSystemInDarkTheme()
            val colorScheme = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                val context = LocalContext.current
                if (darkTheme) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
            } else if (darkTheme) {
                CatppuccinDarkColors
            } else {
                CatppuccinLightColors
            }
            MaterialTheme(colorScheme = colorScheme, typography = AppTypography, shapes = AppShapes) {
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

@OptIn(ExperimentalComposeUiApi::class)
@Composable
private fun NvApp(storage: NvStorage) {
    val scope = rememberCoroutineScope()
    val newNoteDefaultTitle = stringResource(R.string.new_note_default_title)
    val windowSizeClass = WindowSizeClass.fromWidth(LocalConfiguration.current.screenWidthDp)
    val storage = storage
    var notes by remember { mutableStateOf<List<NoteSnapshot>>(emptyList()) }
    var trash by remember { mutableStateOf<List<NoteSnapshot>>(emptyList()) }
    var editorNote by remember { mutableStateOf<NoteSnapshot?>(null) }
    var editorAutoFocus by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var query by remember { mutableStateOf("") }
    var results by remember { mutableStateOf<List<NoteSnapshot>?>(null) }

    fun refresh() = scope.launch {
        withContext(Dispatchers.IO) {
            runCatching {
                val all = storage.listNotes()
                val trashed = storage.listTrash()
                val filtered = if (query.isBlank()) null else storage.searchNotes(query)
                Triple(all, trashed, filtered)
            }
        }.onSuccess { (freshNotes, freshTrash, freshResults) ->
            notes = freshNotes; trash = freshTrash; results = freshResults; error = null
        }.onFailure { error = it.message }
    }

    fun ioOp(block: suspend () -> Unit) {
        scope.launch {
            withContext(Dispatchers.IO) { block() }
            refresh()
        }
    }

    LaunchedEffect(storage) { refresh() }

    // Debounce search: typing updates `query`; this relaunches after 300ms
    // of inactivity so refresh() re-runs searchNotes with the final query.
    LaunchedEffect(query) {
        delay(300)
        refresh()
    }

    val navController = rememberNavController()
    NavHost(
        navController = navController,
        startDestination = "list",
        // Predictive back: scale the exiting screen toward the swipe edge,
        // like the system back-to-home animation that follows the gesture,
        // instead of the default center-origin scale.
        predictivePopExitTransition = { swipeEdge ->
            scaleOut(
                targetScale = 0.7f,
                transformOrigin = TransformOrigin(
                    pivotFractionX = if (swipeEdge == NavigationEvent.EDGE_LEFT) 0f else 1f,
                    pivotFractionY = 0.5f,
                )
            )
        },
    ) {
        composable("list") {
            NotesListScreen(
                notes = results ?: notes,
                filtering = results != null,
                query = query,
                error = error,
                windowSizeClass = windowSizeClass,
                onOpen = { id ->
                    editorNote = notes.firstOrNull { it.id == id }
                    editorAutoFocus = false
                    navController.navigate("editor")
                },
                onQueryChange = { query = it },
                onTrash = { navController.navigate("trash") },
                onCreate = {
                    // createNote is IO; navigation and Compose state must run
                    // on the main thread (NavController touches the lifecycle).
                    scope.launch {
                        val created = withContext(Dispatchers.IO) {
                            runCatching { storage.createNote(newNoteDefaultTitle) }
                        }
                        created.onSuccess { note ->
                            editorNote = note      // editor reads the note BY VALUE after creation
                            editorAutoFocus = true // auto-open the keyboard only on the brand-new note
                            navController.navigate("editor")
                            refresh()
                        }.onFailure { error = it.message }
                    }
                }
            )
        }
        composable("editor") {
            // Refresh the list whenever the editor leaves composition, so a
            // SYSTEM back gesture (which bypasses onDone) still shows fresh data.
            DisposableEffect(Unit) {
                onDispose { refresh() }
            }
            val note = editorNote
            if (note == null) return@composable
            NoteEditorScreen(
                note = note,
                storage = storage,
                windowSizeClass = windowSizeClass,
                autoFocusKeyboard = editorAutoFocus,
                onDone = {
                    editorAutoFocus = false
                    navController.popBackStack()
                },
                onRenamed = { renamed ->
                    editorNote = renamed
                    refresh()
                }
            )
        }
        composable("trash") {
            TrashScreen(
                trash = trash,
                error = error,
                windowSizeClass = windowSizeClass,
                onBack = { navController.popBackStack() },
                onRestore = { id -> ioOp { storage.restoreNote(id) } },
                onPurge = { id -> ioOp { storage.purgeNote(id) } },
                onEmpty = { ioOp { storage.emptyTrash() } }
            )
        }
    }
}
