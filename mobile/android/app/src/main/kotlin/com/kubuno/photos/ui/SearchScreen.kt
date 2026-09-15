package com.kubuno.photos.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.Videocam
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.FilterChip
import androidx.compose.material3.FilterChipDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import com.kubuno.android.account.SharedAccount

private data class Category(val id: String, val label: String, val icon: ImageVector)

private val CATEGORIES = listOf(
    Category("starred", "Favoris", Icons.Filled.Star),
    Category("video", "Vidéos", Icons.Filled.Videocam),
    Category("trashed", "Corbeille", Icons.Filled.Delete),
)

/**
 * Full-screen search over the photothèque: a text query (server `search=`) plus
 * browsable category shortcuts, with a date-sectioned results grid. Opened from
 * the floating round search button.
 */
@Composable
fun SearchScreen(
    account: SharedAccount,
    state: PhotosUiState,
    viewModel: PhotosViewModel,
) {
    Column(
        Modifier
            .fillMaxSize()
            .background(MaterialTheme.colorScheme.surface)
            .statusBarsPadding(),
    ) {
        Row(
            Modifier
                .fillMaxWidth()
                .padding(horizontal = 8.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = viewModel::closeSearch) {
                Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Fermer la recherche")
            }
            TextField(
                value = state.query,
                onValueChange = viewModel::onQuery,
                modifier = Modifier.fillMaxWidth(),
                singleLine = true,
                placeholder = { Text("Rechercher dans les photos") },
                leadingIcon = { Icon(Icons.Filled.Search, contentDescription = null) },
                keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(imeAction = ImeAction.Search),
                shape = PhotosShape.Pill,
                colors = TextFieldDefaults.colors(
                    focusedIndicatorColor = Color.Transparent,
                    unfocusedIndicatorColor = Color.Transparent,
                    disabledIndicatorColor = Color.Transparent,
                ),
            )
        }

        LazyRow(
            Modifier.fillMaxWidth(),
            contentPadding = PaddingValues(horizontal = 16.dp, vertical = 4.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            items(CATEGORIES, key = { it.id }) { cat ->
                FilterChip(
                    selected = state.activeCategory == cat.id,
                    onClick = { viewModel.selectCategory(cat.id) },
                    label = { Text(cat.label) },
                    leadingIcon = { Icon(cat.icon, contentDescription = null, modifier = Modifier.padding(2.dp)) },
                    colors = FilterChipDefaults.filterChipColors(
                        selectedContainerColor = PhotosColors.Blue.copy(alpha = 0.14f),
                        selectedLabelColor = PhotosColors.Blue,
                        selectedLeadingIconColor = PhotosColors.Blue,
                    ),
                )
            }
        }

        Box(Modifier.fillMaxSize()) {
            when {
                state.searchLoading && state.searchSections.isEmpty() ->
                    CircularProgressIndicator(Modifier.align(Alignment.Center))

                state.query.isBlank() && state.activeCategory == null ->
                    Hint("Cherchez par nom de fichier, ou parcourez une catégorie.")

                state.searchSections.isEmpty() ->
                    Hint("Aucun résultat.")

                else -> PhotoGrid(
                    account = account,
                    sections = state.searchSections,
                    density = GridDensity.DAY,
                    selected = emptySet(),
                    selectionMode = false,
                    contentPadding = PaddingValues(top = 4.dp, bottom = 24.dp, start = 2.dp, end = 2.dp),
                    onOpen = { viewModel.openSearchViewer(it.id) },
                    onToggleSelect = {},
                    onToggleSection = {},
                    onDensity = {},
                )
            }
        }
    }
}

@Composable
private fun Hint(text: String) {
    Box(Modifier.fillMaxSize().padding(32.dp), contentAlignment = Alignment.Center) {
        Text(text, color = MaterialTheme.colorScheme.onSurfaceVariant)
    }
}
