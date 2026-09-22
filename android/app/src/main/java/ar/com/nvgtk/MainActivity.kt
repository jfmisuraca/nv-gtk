package ar.com.nvgtk

import android.os.Build
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.animateContentSize
import androidx.compose.animation.AnimatedContentTransitionScope
import androidx.compose.animation.AnimatedContentTransitionScope.SlideDirection
import androidx.compose.animation.EnterTransition
import androidx.compose.animation.ExitTransition
import androidx.compose.animation.core.Animatable
import androidx.compose.animation.core.Spring
import androidx.compose.animation.core.spring
import androidx.compose.animation.core.tween
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
import androidx.compose.material3.ExperimentalMaterial3Api
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

@OptIn(ExperimentalMaterial3Api::class, ExperimentalComposeUiApi::class)
@Composable
private fun NvApp(storage: NvStorage) {
    val scope = rememberCoroutineScope()
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
    // Sliding transitions match the platform back animation: on pop the whole
    // screen slides right following the gesture (previous screen static
    // beneath), instead of Navigation's default cross-fade. Forward nav slides
    // the new screen in from the right, same as the system stack push.
    NavHost(
        navController = navController,
        startDestination = "list",
        enterTransition = {
            slideIntoContainer(
                animationSpec = tween(300),
                towards = SlideDirection.Left
            )
        },
        exitTransition = { ExitTransition.None },
        popEnterTransition = { EnterTransition.None },
        popExitTransition = {
            slideOutOfContainer(
                animationSpec = tween(300),
                towards = SlideDirection.Right
            )
        }
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
                    ioOp {
                        val created = storage.createNote("Nota nueva")
                        editorNote = created  // editor reads the note BY VALUE after creation
                        editorAutoFocus = true // auto-open the keyboard only on the brand-new note
                        navController.navigate("editor")
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
