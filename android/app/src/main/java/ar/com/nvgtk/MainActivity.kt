package ar.com.nvgtk

import android.content.res.Configuration
import android.graphics.drawable.ColorDrawable
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
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.navigation.compose.NavHost
import androidx.navigation.compose.composable
import androidx.navigation.compose.rememberNavController
import androidx.navigation.NavOptionsBuilder
import androidx.navigation.NavType
import androidx.navigation.navArgument
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import uniffi.nv_core.NoteSnapshot
import uniffi.nv_core.NvStorage
import java.io.File
import java.time.LocalDateTime
import java.time.format.DateTimeFormatter

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

/** T7: editor route for a note id; distinct ids push distinct back-stack
 * entries, so back walks lista→A→B→A→lista instead of flattening. */
internal fun editorRoute(noteId: String): String = "editor/$noteId"

/** T7: trimmed link target, or null when blank (nothing to follow). */
internal fun cleanWikiTarget(target: String): String? =
    target.trim().takeIf { it.isNotEmpty() }

/** T7-fix: THE options object every `navigate()` to an editor route uses
 * (passed as `::applyEditorNavOptions`, so the regression test locks the
 * real call's options, not a copy). One entry per note: `launchSingleTop`
 * would reuse the top entry and flatten A→B; any `popUpTo` would drop A;
 * both would turn back from B into back-to-lista. */
internal fun applyEditorNavOptions(options: NavOptionsBuilder) {
    options.launchSingleTop = false
    options.restoreState = false
}

/** T7-fix: follow-link routing decision used verbatim by `followWikiLink`.
 * Returns the route to push, or null for a self-link (stay in place). */
internal fun editorFollowRoute(currentId: String?, targetId: String): String? =
    if (targetId == currentId) null else editorRoute(targetId)

class MainActivity : ComponentActivity() {

    private lateinit var storage: NvStorage

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()
        // T10: paint the window with the active palette's base color before
        // setContent, so the static day/night XML background never flashes
        // the wrong palette on boot or day/night recreation.
        val bootTheme = ThemePrefs.load(this)
        val bootDark =
            (resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) ==
                Configuration.UI_MODE_NIGHT_YES
        window.setBackgroundDrawable(ColorDrawable(bootBackground(bootTheme, bootDark).toArgb()))
        val notesDir = File(filesDir, "notes").apply { mkdirs() }
        storage = NvStorage.open(notesDir.absolutePath)
        setContent {
            // T9: theme choice is hoisted here, above MaterialTheme, so the
            // selected palette applies live across every route. Persisted via
            // ThemePrefs (SharedPreferences, no new dependency); default
            // Wallpaper resolves to Catppuccin below API 31.
            val context = LocalContext.current
            var theme by remember { mutableStateOf(ThemePrefs.load(context)) }
            val darkTheme = isSystemInDarkTheme()
            val colorScheme = colorSchemeFor(theme, darkTheme)
            MaterialTheme(colorScheme = colorScheme, typography = AppTypography, shapes = AppShapes) {
                Surface(Modifier.fillMaxSize()) {
                    NvApp(
                        storage,
                        theme = theme,
                        onThemeChange = { selected ->
                            theme = selected
                            ThemePrefs.save(context, selected)
                        }
                    )
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
private fun NvApp(
    storage: NvStorage,
    theme: NvTheme,
    onThemeChange: (NvTheme) -> Unit
) {
    val scope = rememberCoroutineScope()
    val windowSizeClass = WindowSizeClass.fromWidth(LocalConfiguration.current.screenWidthDp)
    val storage = storage
    var notes by remember { mutableStateOf<List<NoteSnapshot>>(emptyList()) }
    var trash by remember { mutableStateOf<List<NoteSnapshot>>(emptyList()) }
    // T7: per-note snapshots keyed by id, so each `editor/{noteId}`
    // back-stack entry resolves its own note (back returns to the previous
    // note instead of a shared value overwritten by the last navigation).
    var editorSnapshots by remember { mutableStateOf<Map<String, NoteSnapshot>>(emptyMap()) }
    var editorAutoFocus by remember { mutableStateOf(false) }
    // T7-fix: true once storage data has loaded. The editor entry below
    // self-pops only when this is true AND its id is still unknown; before
    // the first load (e.g. activity recreation restores the NavController
    // stack while notes reload) entries must wait, not pop.
    var notesLoaded by remember { mutableStateOf(false) }
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
            notesLoaded = true
        }.onFailure {
            error = it.message
            // A failed load will not resolve entries later, so let unknown
            // ids fall through to the list (which shows the error) instead
            // of waiting on a blank frame forever.
            notesLoaded = true
        }
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

    // Wiki-link navigation (parity slice 3, desktop flow): open the note
    // whose display title matches `target`; when none matches, create a note
    // with the target as content (like desktop Enter-on-search) and open it.
    // T7: pushes one back-stack entry per note (no single-top flattening), so
    // system back returns to the previous note; a self-link stays in place.
    fun followWikiLink(target: String) {
        val clean = cleanWikiTarget(target) ?: return
        scope.launch {
            val resolved = withContext(Dispatchers.IO) {
                runCatching {
                    val existing = storage.wikiResolve(clean)
                    if (existing != null) {
                        existing
                    } else {
                        // Desktop-parity timestamp stem (yyyyMMdd-HHmm); the
                        // core disambiguates same-minute collisions.
                        val stamp = LocalDateTime.now()
                            .format(DateTimeFormatter.ofPattern("yyyyMMdd-HHmm"))
                        val created = storage.createNote(stamp)
                        storage.saveNote(created.id, clean)
                    }
                }
            }
            resolved.onSuccess { snap ->
                editorSnapshots = editorSnapshots + (snap.id to snap)
                editorAutoFocus = false
                val currentId = navController.currentBackStackEntry
                    ?.arguments?.getString("noteId")
                val route = editorFollowRoute(currentId, snap.id)
                if (route != null) {
                    navController.navigate(route, ::applyEditorNavOptions)
                }
                refresh()
            }.onFailure { error = it.message }
        }
    }
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
                    (results ?: notes).firstOrNull { it.id == id }?.let { found ->
                        editorSnapshots = editorSnapshots + (found.id to found)
                        editorAutoFocus = false
                        navController.navigate(editorRoute(found.id), ::applyEditorNavOptions)
                    }
                },
                onQueryChange = { query = it },
                onTrash = { navController.navigate("trash") },
                onSettings = { navController.navigate("settings") },
                onCreate = {
                    // createNote is IO; navigation and Compose state must run
                    // on the main thread (NavController touches the lifecycle).
                    scope.launch {
                        val created = withContext(Dispatchers.IO) {
                            runCatching {
                                // Desktop-parity timestamp stem (AAAAMMDD-HHMM);
                                // core disambiguates same-minute collisions
                                // with seconds. java.time needs API 26+ and
                                // minSdk is already 26.
                                val stamp = LocalDateTime.now()
                                    .format(DateTimeFormatter.ofPattern("yyyyMMdd-HHmm"))
                                storage.createNote(stamp)
                            }
                        }
                        created.onSuccess { note ->
                            editorSnapshots = editorSnapshots + (note.id to note)
                            editorAutoFocus = true // auto-open the keyboard only on the brand-new note
                            navController.navigate(editorRoute(note.id), ::applyEditorNavOptions)
                            refresh()
                        }.onFailure { error = it.message }
                    }
                }
            )
        }
        composable(
            route = "editor/{noteId}",
            arguments = listOf(navArgument("noteId") { type = NavType.StringType })
        ) { backStackEntry ->
            // Refresh the list whenever the editor leaves composition, so a
            // SYSTEM back gesture (which bypasses onDone) still shows fresh data.
            DisposableEffect(Unit) {
                onDispose { refresh() }
            }
            val noteId = backStackEntry.arguments?.getString("noteId")
            // Each stack entry resolves its own note: the cached snapshot
            // first (covers just-created notes before refresh lands), then
            // the fresh list.
            val note = editorSnapshots[noteId]
                ?: notes.firstOrNull { it.id == noteId }
                ?: results?.firstOrNull { it.id == noteId }
            if (note == null) {
                // T7-fix: pop ONLY once storage data has loaded and the id is
                // still unknown (e.g. a deleted note). While loading — notably
                // after activity recreation, when rememberNavController
                // restores entries but snapshots/notes are not back yet — the
                // entry waits instead of popping itself: popping here collapses
                // valid entries to lista and back can never return to them.
                if (notesLoaded) {
                    LaunchedEffect(Unit) { navController.popBackStack() }
                }
                return@composable
            }
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
                    // A rename changes the file/id: keep both the pre-rename
                    // route id and the new id pointing at the snapshot.
                    editorSnapshots = editorSnapshots +
                        (note.id to renamed) + (renamed.id to renamed)
                    refresh()
                },
                onFollowLink = { target -> followWikiLink(target) }
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
        composable("settings") {
            SettingsScreen(
                selected = theme,
                onSelect = onThemeChange,
                onBack = { navController.popBackStack() }
            )
        }
    }
}
