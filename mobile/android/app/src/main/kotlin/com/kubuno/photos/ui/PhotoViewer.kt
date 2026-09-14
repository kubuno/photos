package com.kubuno.photos.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.StarBorder
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.IntOffset
import androidx.compose.ui.unit.dp
import coil3.compose.AsyncImage
import coil3.request.ImageRequest
import coil3.request.crossfade
import com.kubuno.android.account.SharedAccount
import com.kubuno.photos.net.PhotoDto
import java.time.OffsetDateTime
import java.time.format.DateTimeFormatter
import java.util.Locale
import kotlin.math.roundToInt

/**
 * Full-screen photo viewer — a horizontal pager over the whole roll, a glanceable
 * date up top, and the reference action bar (Share · Favorite · Delete) at the
 * bottom. Swipe down to dismiss back to the grid.
 */
@Composable
fun PhotoViewer(
    account: SharedAccount,
    photos: List<PhotoDto>,
    startIndex: Int,
    onClose: () -> Unit,
    onToggleStar: (String) -> Unit,
    onShare: (PhotoDto) -> Unit,
    onDelete: (PhotoDto) -> Unit,
) {
    if (photos.isEmpty()) {
        onClose()
        return
    }
    val context = LocalContext.current
    val pagerState = rememberPagerState(
        initialPage = startIndex.coerceIn(0, photos.lastIndex),
        pageCount = { photos.size },
    )
    val current = photos.getOrNull(pagerState.currentPage) ?: photos.first()

    Box(
        Modifier
            .fillMaxSize()
            .background(Color.Black),
    ) {
        // Vertical drag on the whole surface dismisses when pulled down far enough.
        var dragY by remember { mutableFloatStateOf(0f) }
        HorizontalPager(
            state = pagerState,
            modifier = Modifier
                .fillMaxSize()
                .offset { IntOffset(0, dragY.roundToInt()) }
                .pointerInput(Unit) {
                    detectVerticalDragGestures(
                        onDragEnd = {
                            if (dragY > 220f) onClose() else dragY = 0f
                        },
                        onVerticalDrag = { _, delta ->
                            dragY = (dragY + delta).coerceAtLeast(0f)
                        },
                    )
                },
        ) { page ->
            val photo = photos[page]
            AsyncImage(
                model = ImageRequest.Builder(context)
                    .data(previewUrl(account, photo.id))
                    .crossfade(true)
                    .build(),
                contentDescription = photo.originalName ?: photo.filename,
                contentScale = ContentScale.Fit,
                modifier = Modifier.fillMaxSize(),
            )
        }

        // Top bar: back + glanceable date/time.
        Row(
            Modifier
                .fillMaxWidth()
                .background(Color.Black.copy(alpha = 0.35f))
                .statusBarsPadding()
                .padding(horizontal = 4.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onClose) {
                Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Retour", tint = Color.White)
            }
            Column(Modifier.weight(1f)) {
                Text(
                    text = glanceDate(current),
                    color = Color.White,
                    style = androidx.compose.material3.MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.SemiBold,
                )
                glanceTime(current)?.let {
                    Text(it, color = Color.White.copy(alpha = 0.8f), style = androidx.compose.material3.MaterialTheme.typography.bodySmall)
                }
            }
            IconButton(onClick = { /* info sheet lands in a later milestone */ }) {
                Icon(Icons.Outlined.Info, contentDescription = "Informations", tint = Color.White)
            }
        }

        // Bottom action bar.
        Row(
            Modifier
                .align(Alignment.BottomCenter)
                .fillMaxWidth()
                .background(Color.Black.copy(alpha = 0.35f))
                .navigationBarsPadding()
                .padding(vertical = 6.dp),
            horizontalArrangement = Arrangement.SpaceEvenly,
            verticalAlignment = Alignment.CenterVertically,
        ) {
            ViewerAction(Icons.Filled.Share, "Partager") { onShare(current) }
            ViewerAction(
                if (current.isStarred) Icons.Filled.Star else Icons.Filled.StarBorder,
                "Favori",
            ) { onToggleStar(current.id) }
            ViewerAction(Icons.Filled.Delete, "Supprimer") { onDelete(current) }
        }
    }
}

@Composable
private fun ViewerAction(icon: androidx.compose.ui.graphics.vector.ImageVector, label: String, onClick: () -> Unit) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        IconButton(onClick = onClick) {
            Icon(icon, contentDescription = label, tint = Color.White)
        }
        Text(label, color = Color.White, style = androidx.compose.material3.MaterialTheme.typography.labelSmall)
    }
}

private val DATE_FMT = DateTimeFormatter.ofPattern("EEE d MMMM yyyy", Locale.FRENCH)
private val TIME_FMT = DateTimeFormatter.ofPattern("HH:mm", Locale.FRENCH)

private fun glanceDate(photo: PhotoDto): String {
    val iso = photo.instant ?: return "Sans date"
    return runCatching { OffsetDateTime.parse(iso).format(DATE_FMT).replaceFirstChar { it.uppercase() } }
        .getOrElse { iso.take(10) }
}

private fun glanceTime(photo: PhotoDto): String? {
    val iso = photo.instant ?: return null
    return runCatching { OffsetDateTime.parse(iso).format(TIME_FMT) }.getOrNull()
}
