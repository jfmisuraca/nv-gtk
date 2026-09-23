package ar.com.nvgtk

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.unit.dp

/**
 * Theme picker (T9): lists the four [NvTheme] palettes as grouped radio
 * options. Selection is applied live by the caller and persisted via
 * [ThemePrefs]; this screen holds no state of its own.
 *
 * A11y (M3 Expressive T7 convention): the options live in a
 * [selectableGroup] and each row is `selectable` with [Role.RadioButton],
 * so TalkBack announces position and checked state. Rows are at least
 * 48dp tall for touch targets.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SettingsScreen(
    selected: NvTheme,
    onSelect: (NvTheme) -> Unit,
    onBack: () -> Unit
) {
    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(stringResource(R.string.settings_title)) },
                navigationIcon = {
                    IconButton(onClick = onBack) {
                        Icon(Icons.Filled.ArrowBack, contentDescription = stringResource(R.string.back))
                    }
                }
            )
        }
    ) { padding ->
        Column(
            Modifier
                .padding(padding)
                .fillMaxSize()
                .selectableGroup()
        ) {
            Text(
                stringResource(R.string.theme_section_title),
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.padding(16.dp)
            )
            for (theme in NvTheme.entries) {
                val label = when (theme) {
                    NvTheme.Catppuccin -> stringResource(R.string.theme_catppuccin)
                    NvTheme.Dracula -> stringResource(R.string.theme_dracula)
                    NvTheme.Flexoki -> stringResource(R.string.theme_flexoki)
                    NvTheme.Wallpaper -> stringResource(R.string.theme_wallpaper)
                }
                Row(
                    modifier = Modifier
                        .fillMaxWidth()
                        .heightIn(min = 48.dp)
                        .selectable(
                            selected = theme == selected,
                            onClick = { onSelect(theme) },
                            role = Role.RadioButton
                        )
                        .padding(horizontal = 16.dp, vertical = 8.dp),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    RadioButton(
                        selected = theme == selected,
                        onClick = null
                    )
                    Text(
                        label,
                        style = MaterialTheme.typography.bodyLarge,
                        modifier = Modifier.padding(start = 16.dp)
                    )
                }
            }
        }
    }
}
