package com.kubuno.photos.ui

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.gestures.detectVerticalDragGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.pager.HorizontalPager
import androidx.compose.foundation.pager.rememberPagerState
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Place
import androidx.compose.material.icons.filled.Share
import androidx.compose.material.icons.filled.Star
import androidx.compose.material.icons.filled.StarBorder
import androidx.compose.material.icons.automirrored.outlined.Notes
import androidx.compose.material.icons.outlined.CalendarMonth
import androidx.compose.material.icons.outlined.Image
import androidx.compose.material.icons.outlined.Info
import androidx.compose.material.icons.outlined.PhotoCamera
import androidx.compose.material.icons.outlined.Place
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
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
import kotlin.math.cos
import kotlin.math.floor
import kotlin.math.ln
import kotlin.math.roundToInt
import kotlin.math.tan

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

    var showInfo by remember { mutableStateOf(false) }
    Box(
        Modifier
            .fillMaxSize()
            .background(Color.Black),
    ) {
        // Vertical drag: pull down far enough to dismiss, pull up to open the
        // info sheet — the reference gallery's two swipe directions.
        var dragY by remember { mutableFloatStateOf(0f) }
        HorizontalPager(
            state = pagerState,
            modifier = Modifier
                .fillMaxSize()
                // Only the downward pull moves the image; an upward pull just
                // accumulates toward opening the sheet.
                .offset { IntOffset(0, dragY.coerceAtLeast(0f).roundToInt()) }
                .pointerInput(Unit) {
                    detectVerticalDragGestures(
                        onDragEnd = {
                            when {
                                dragY > 220f -> onClose()
                                dragY < -120f -> showInfo = true
                            }
                            dragY = 0f
                        },
                        onVerticalDrag = { _, delta -> dragY += delta },
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
            IconButton(onClick = { showInfo = true }) {
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

        if (showInfo) {
            PhotoInfoSheet(photo = current, onDismiss = { showInfo = false })
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun PhotoInfoSheet(photo: PhotoDto, onDismiss: () -> Unit) {
    ModalBottomSheet(onDismissRequest = onDismiss) {
        Column(
            Modifier
                .fillMaxWidth()
                .padding(horizontal = 20.dp)
                .padding(bottom = 28.dp),
        ) {
            Text(
                "Détails",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.padding(bottom = 8.dp),
            )
            InfoRow(Icons.Outlined.CalendarMonth, glanceDate(photo), glanceTime(photo))
            InfoRow(
                Icons.Outlined.Image,
                photo.originalName ?: photo.filename ?: "Sans nom",
                detailsLine(photo),
            )
            cameraLine(photo)?.let { InfoRow(Icons.Outlined.PhotoCamera, it, null) }
            if (photo.gpsLat != null && photo.gpsLon != null) {
                InfoRow(
                    Icons.Outlined.Place,
                    "Lieu",
                    "%.5f, %.5f".format(java.util.Locale.US, photo.gpsLat, photo.gpsLon),
                )
                MiniMap(photo.gpsLat, photo.gpsLon)
            }
            photo.description?.takeIf { it.isNotBlank() }?.let {
                InfoRow(Icons.AutoMirrored.Outlined.Notes, "Description", it)
            }
        }
    }
}

@Composable
private fun InfoRow(icon: androidx.compose.ui.graphics.vector.ImageVector, title: String, subtitle: String?) {
    Row(
        Modifier
            .fillMaxWidth()
            .padding(vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Icon(
            icon,
            contentDescription = null,
            tint = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.size(22.dp),
        )
        Column(Modifier.padding(start = 18.dp)) {
            Text(title, style = MaterialTheme.typography.bodyLarge)
            subtitle?.let {
                Text(it, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
    }
}

/** "1653 × 2122 · 6,1 Mo · PNG" — whichever parts are known. */
private fun detailsLine(photo: PhotoDto): String? {
    val parts = buildList {
        if (photo.width != null && photo.height != null) add("${photo.width} × ${photo.height}")
        if (photo.sizeBytes > 0) add(formatSize(photo.sizeBytes))
        photo.mimeType?.substringAfter('/')?.uppercase()?.let { add(it) }
    }
    return parts.takeIf { it.isNotEmpty() }?.joinToString(" · ")
}

private fun cameraLine(photo: PhotoDto): String? {
    val text = listOfNotNull(photo.cameraMake, photo.cameraModel).joinToString(" ").trim()
    return text.ifBlank { null }
}

private fun formatSize(bytes: Long): String = when {
    bytes >= 1_000_000 -> String.format(java.util.Locale.FRENCH, "%.1f Mo", bytes / 1_000_000.0)
    else -> String.format(java.util.Locale.FRENCH, "%d Ko", bytes / 1000)
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

/**
 * A small static map centred on the photo's GPS point, with a pin. Uses keyless
 * ESRI raster tiles (no user-agent restrictions, unlike the OSM tile server) and
 * places the pin at the point's fractional position inside the tile. Tapping
 * opens the location in a maps app via a standard geo: link.
 */
@Composable
private fun MiniMap(lat: Double, lon: Double) {
    val context = LocalContext.current
    val z = 14
    val n = (1 shl z).toDouble()
    val latRad = Math.toRadians(lat)
    val xf = (lon + 180.0) / 360.0 * n
    val yf = (1.0 - ln(tan(latRad) + 1.0 / cos(latRad)) / Math.PI) / 2.0 * n
    val xTile = floor(xf).toInt()
    val yTile = floor(yf).toInt()
    val fx = (xf - xTile).toFloat()
    val fy = (yf - yTile).toFloat()
    // ESRI tiles are addressed {z}/{y}/{x}.
    val url = "https://server.arcgisonline.com/ArcGIS/rest/services/World_Street_Map/MapServer/tile/$z/$yTile/$xTile"
    val pin = 30.dp
    BoxWithConstraints(
        Modifier
            .padding(vertical = 8.dp)
            .fillMaxWidth()
            .height(160.dp)
            .clip(RoundedCornerShape(14.dp))
            .clickable {
                runCatching {
                    context.startActivity(
                        android.content.Intent(
                            android.content.Intent.ACTION_VIEW,
                            android.net.Uri.parse("geo:$lat,$lon?q=$lat,$lon"),
                        ),
                    )
                }
            },
    ) {
        AsyncImage(
            model = ImageRequest.Builder(context).data(url).crossfade(true).build(),
            contentDescription = "Carte du lieu",
            contentScale = ContentScale.FillBounds,
            modifier = Modifier.fillMaxSize(),
        )
        Icon(
            Icons.Filled.Place,
            contentDescription = null,
            tint = PhotosColors.PetalRed,
            modifier = Modifier
                .align(Alignment.TopStart)
                // Anchor the pin tip (bottom-centre) on the point.
                .offset(x = maxWidth * fx - pin / 2f, y = maxHeight * fy - pin)
                .size(pin),
        )
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
