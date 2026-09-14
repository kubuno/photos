package com.kubuno.photos.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.combinedClickable
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.foundation.gestures.calculateZoom
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.PlayCircle
import androidx.compose.material.icons.filled.Star
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import coil3.request.ImageRequest
import coil3.request.crossfade
import com.kubuno.android.account.SharedAccount
import com.kubuno.photos.net.PhotoDto

/**
 * The date-sectioned photo grid — reverse-chronological, sticky-feeling date
 * headers, square cropped tiles, pinch-to-zoom between the density levels.
 * Long-press starts multi-selection; a header tap selects the whole day.
 */
@Composable
fun PhotoGrid(
    account: SharedAccount,
    sections: List<PhotoSection>,
    density: GridDensity,
    selected: Set<String>,
    selectionMode: Boolean,
    contentPadding: PaddingValues,
    onOpen: (PhotoDto) -> Unit,
    onToggleSelect: (String) -> Unit,
    onToggleSection: (PhotoSection) -> Unit,
    onDensity: (GridDensity) -> Unit,
    modifier: Modifier = Modifier,
) {
    val gridState = rememberLazyGridState()
    LazyVerticalGrid(
        columns = GridCells.Fixed(density.columns),
        state = gridState,
        modifier = modifier
            .fillMaxSize()
            .pinchToZoom(
                onZoomIn = { onDensity(density.zoomIn()) },
                onZoomOut = { onDensity(density.zoomOut()) },
            ),
        contentPadding = contentPadding,
        horizontalArrangement = Arrangement.spacedBy(2.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        sections.forEach { section ->
            item(key = "h_${section.key}", span = { GridItemSpan(maxLineSpan) }) {
                SectionHeader(
                    section = section,
                    selected = section.photos.all { it.id in selected } && section.photos.isNotEmpty(),
                    selectionMode = selectionMode,
                    onClick = { onToggleSection(section) },
                )
            }
            items(section.photos, key = { it.id }) { photo ->
                PhotoTile(
                    account = account,
                    photo = photo,
                    selected = photo.id in selected,
                    selectionMode = selectionMode,
                    onClick = {
                        if (selectionMode) onToggleSelect(photo.id) else onOpen(photo)
                    },
                    onLongClick = { onToggleSelect(photo.id) },
                )
            }
        }
    }
}

@Composable
private fun SectionHeader(
    section: PhotoSection,
    selected: Boolean,
    selectionMode: Boolean,
    onClick: () -> Unit,
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(8.dp))
            .combinedClickable(onClick = onClick)
            .padding(start = 6.dp, end = 12.dp, top = 16.dp, bottom = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = section.header,
            style = MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.padding(start = 6.dp),
        )
        if (selectionMode) {
            Box(Modifier.fillMaxWidth(), contentAlignment = Alignment.CenterEnd) {
                val tint = if (selected) PhotosColors.Blue else MaterialTheme.colorScheme.outline
                Icon(
                    Icons.Filled.CheckCircle,
                    contentDescription = "Sélectionner le groupe",
                    tint = tint,
                    modifier = Modifier.size(20.dp),
                )
            }
        }
    }
}

@Composable
private fun PhotoTile(
    account: SharedAccount,
    photo: PhotoDto,
    selected: Boolean,
    selectionMode: Boolean,
    onClick: () -> Unit,
    onLongClick: () -> Unit,
) {
    val context = androidx.compose.ui.platform.LocalContext.current
    Box(
        modifier = Modifier
            .aspectRatio(1f)
            .combinedClickable(onClick = onClick, onLongClick = onLongClick)
            // The selected tile pulls inward to reveal the pinwheel-blue ground.
            .then(if (selected) Modifier.padding(6.dp) else Modifier)
            .clip(if (selected) RoundedCornerShape(10.dp) else RoundedCornerShape(0.dp))
            .background(MaterialTheme.colorScheme.surfaceVariant),
    ) {
        AsyncImage(
            model = ImageRequest.Builder(context)
                .data(thumbUrl(account, photo.id))
                .crossfade(true)
                .build(),
            contentDescription = photo.originalName ?: photo.filename,
            contentScale = ContentScale.Crop,
            modifier = Modifier.fillMaxSize(),
        )
        if (photo.isVideo) {
            Icon(
                Icons.Filled.PlayCircle,
                contentDescription = "Vidéo",
                tint = Color.White,
                modifier = Modifier.align(Alignment.Center).size(34.dp),
            )
        }
        if (photo.isStarred) {
            Icon(
                Icons.Filled.Star,
                contentDescription = "Favori",
                tint = Color.White,
                modifier = Modifier
                    .align(Alignment.BottomStart)
                    .padding(4.dp)
                    .size(16.dp),
            )
        }
        if (selectionMode) {
            val tint = if (selected) PhotosColors.Blue else Color.White
            Icon(
                Icons.Filled.CheckCircle,
                contentDescription = if (selected) "Sélectionné" else "Sélectionner",
                tint = tint,
                modifier = Modifier
                    .align(Alignment.TopStart)
                    .padding(4.dp)
                    .size(22.dp)
                    .clip(CircleShape),
            )
        }
    }
}

/**
 * Pinch anywhere on the grid to step the density. Only consumes when two or
 * more pointers are down, so a one-finger scroll still reaches the grid.
 */
private fun Modifier.pinchToZoom(
    onZoomIn: () -> Unit,
    onZoomOut: () -> Unit,
): Modifier = pointerInput(Unit) {
    awaitEachGesture {
        awaitFirstDown(requireUnconsumed = false)
        var zoom = 1f
        var fired = false
        do {
            val event = awaitPointerEvent()
            val pressed = event.changes.count { it.pressed }
            if (pressed >= 2) {
                zoom *= event.calculateZoom()
                event.changes.forEach { it.consume() }
                if (!fired && zoom > 1.25f) {
                    onZoomIn(); fired = true
                } else if (!fired && zoom < 0.8f) {
                    onZoomOut(); fired = true
                }
            }
        } while (event.changes.any { it.pressed })
    }
}
