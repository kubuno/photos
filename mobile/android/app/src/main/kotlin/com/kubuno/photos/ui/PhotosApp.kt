package com.kubuno.photos.ui

import android.widget.Toast
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.AddCircleOutline
import androidx.compose.material.icons.filled.AddPhotoAlternate
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.PhotoCamera
import androidx.compose.material.icons.filled.PhotoLibrary
import androidx.compose.material.icons.filled.Search
import androidx.compose.material.icons.filled.Videocam
import androidx.compose.material.icons.outlined.PhotoLibrary
import androidx.compose.material.icons.outlined.Collections
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.kubuno.android.account.SharedAccount
import com.kubuno.photos.net.AlbumDto

@Composable
fun PhotosApp(viewModel: PhotosViewModel = hiltViewModel()) {
    val state by viewModel.state.collectAsStateWithLifecycle()
    val context = LocalContext.current

    val account = state.account
    if (state.ready && account == null) {
        NoAccount()
        return
    }
    if (account == null) return

    // Camera capture → upload. We create the output file ourselves (cacheDir),
    // hand the camera app a FileProvider URI to write into, then read the bytes
    // back and upload them. No CAMERA permission: the system camera app owns it.
    var pendingCapture by remember { mutableStateOf<java.io.File?>(null) }
    fun consumeCapture(ok: Boolean, mime: String) {
        val file = pendingCapture
        pendingCapture = null
        if (ok && file != null) {
            val bytes = runCatching { file.readBytes() }.getOrNull()
            if (bytes != null) viewModel.uploadCapture(bytes, mime, file.name)
        }
        file?.delete()
    }
    val takePhoto = rememberLauncherForActivityResult(ActivityResultContracts.TakePicture()) {
        consumeCapture(it, "image/jpeg")
    }
    val takeVideo = rememberLauncherForActivityResult(ActivityResultContracts.CaptureVideo()) {
        consumeCapture(it, "video/mp4")
    }
    fun capture(extension: String, launch: (android.net.Uri) -> Unit) {
        val file = createCaptureFile(context, extension)
        pendingCapture = file
        launch(captureUri(context, file))
    }

    // Import existing photos/videos from the device via the system photo picker
    // (no storage permission). Their bytes are read and uploaded as a batch.
    val importMedia = rememberLauncherForActivityResult(
        ActivityResultContracts.PickMultipleVisualMedia(10),
    ) { uris ->
        if (uris.isNotEmpty()) {
            val items = uris.mapNotNull { uri ->
                val bytes = runCatching {
                    context.contentResolver.openInputStream(uri)?.use { it.readBytes() }
                }.getOrNull() ?: return@mapNotNull null
                val name = displayName(context, uri) ?: "import_${System.currentTimeMillis()}"
                val mime = context.contentResolver.getType(uri) ?: "application/octet-stream"
                UploadItem(bytes, mime, name)
            }
            viewModel.uploadBatch(items)
        }
    }

    // Toasts for upload outcome.
    LaunchedEffect(state.message) {
        state.message?.let {
            Toast.makeText(context, it, Toast.LENGTH_SHORT).show()
            viewModel.clearMessage()
        }
    }

    // The full-screen viewer takes over everything when a photo is open.
    val viewerIndex = state.viewerIndex
    if (viewerIndex != null) {
        PhotoViewer(
            account = account,
            photos = state.photos,
            startIndex = viewerIndex,
            onClose = viewModel::closeViewer,
            onToggleStar = viewModel::toggleStar,
            onShare = { Toast.makeText(context, "Partage bientôt disponible", Toast.LENGTH_SHORT).show() },
            onDelete = { photo ->
                viewModel.toggleSelect(photo.id)
                viewModel.trashSelection()
                viewModel.closeViewer()
            },
        )
        return
    }

    val selectionMode = state.selected.isNotEmpty()

    Box(Modifier.fillMaxSize()) {
        // Content per tab. The grid reserves bottom room for the floating pill.
        when (state.tab) {
            PhotosTab.PHOTOS -> PhotosTabContent(account, state, viewModel, selectionMode)
            PhotosTab.COLLECTIONS -> CollectionsTabContent(state.albums)
            PhotosTab.CREATE -> CreateTabContent(
                onTakePhoto = { capture("jpg") { takePhoto.launch(it) } },
                onTakeVideo = { capture("mp4") { takeVideo.launch(it) } },
                onImport = {
                    importMedia.launch(
                        PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageAndVideo),
                    )
                },
            )
        }

        // Top bar: the account avatar when idle, a selection action bar otherwise.
        if (selectionMode) {
            SelectionBar(
                count = state.selected.size,
                onClose = viewModel::clearSelection,
                onDelete = viewModel::trashSelection,
                modifier = Modifier.align(Alignment.TopCenter),
            )
        } else if (state.tab == PhotosTab.PHOTOS) {
            TopBar(account = account, modifier = Modifier.align(Alignment.TopCenter))
        }

        // Floating navigation pill + detached round search button.
        if (!selectionMode) {
            NavPill(
                tab = state.tab,
                onSelect = viewModel::selectTab,
                onSearch = { Toast.makeText(context, "Recherche bientôt disponible", Toast.LENGTH_SHORT).show() },
                modifier = Modifier
                    .align(Alignment.BottomCenter)
                    .navigationBarsPadding()
                    .padding(bottom = 12.dp),
            )
        }

        // Upload scrim — blocks input and shows progress while a capture uploads.
        if (state.uploading) {
            Box(
                Modifier
                    .fillMaxSize()
                    .background(Color.Black.copy(alpha = 0.4f))
                    .clickable(enabled = false) {},
                contentAlignment = Alignment.Center,
            ) {
                Column(horizontalAlignment = Alignment.CenterHorizontally) {
                    CircularProgressIndicator(color = Color.White)
                    Spacer(Modifier.size(12.dp))
                    Text("Envoi…", color = Color.White)
                }
            }
        }
    }
}

/** A fresh, unique output file for a capture, under cacheDir/captures. */
private fun createCaptureFile(context: android.content.Context, extension: String): java.io.File {
    val dir = java.io.File(context.cacheDir, "captures").apply { mkdirs() }
    return java.io.File(dir, "capture_${System.currentTimeMillis()}.$extension")
}

/** A grantable content URI for [file], for the camera app to write into. */
private fun captureUri(context: android.content.Context, file: java.io.File): android.net.Uri =
    androidx.core.content.FileProvider.getUriForFile(
        context,
        "${context.packageName}.fileprovider",
        file,
    )

/** The display name of a picked content URI, for the uploaded file name. */
private fun displayName(context: android.content.Context, uri: android.net.Uri): String? =
    runCatching {
        context.contentResolver.query(uri, null, null, null, null)?.use { c ->
            val idx = c.getColumnIndex(android.provider.OpenableColumns.DISPLAY_NAME)
            if (idx >= 0 && c.moveToFirst()) c.getString(idx) else null
        }
    }.getOrNull()

@Composable
private fun PhotosTabContent(
    account: SharedAccount,
    state: PhotosUiState,
    viewModel: PhotosViewModel,
    selectionMode: Boolean,
) {
    Box(Modifier.fillMaxSize()) {
        when {
            state.loading && state.sections.isEmpty() ->
                CircularProgressIndicator(Modifier.align(Alignment.Center))

            state.error != null && state.sections.isEmpty() ->
                EmptyState("Impossible de charger la photothèque.\n${state.error}", onRetry = viewModel::refresh)

            state.sections.isEmpty() ->
                EmptyState("Aucune photo pour l'instant.", onRetry = viewModel::refresh)

            else -> PhotoGrid(
                account = account,
                sections = state.sections,
                density = state.density,
                selected = state.selected,
                selectionMode = selectionMode,
                // Leave room for the top bar and the floating pill.
                contentPadding = PaddingValues(top = 72.dp, bottom = 96.dp, start = 2.dp, end = 2.dp),
                onOpen = { viewModel.openViewer(it.id) },
                onToggleSelect = viewModel::toggleSelect,
                onToggleSection = viewModel::toggleSection,
                onDensity = viewModel::setDensity,
            )
        }
    }
}

@Composable
private fun TopBar(account: SharedAccount, modifier: Modifier = Modifier) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .statusBarsPadding()
            .padding(horizontal = 16.dp, vertical = 10.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text(
            text = "Photos",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.SemiBold,
            color = MaterialTheme.colorScheme.onSurface,
            modifier = Modifier.weight(1f),
        )
        AccountAvatar(account)
    }
}

@Composable
private fun SelectionBar(
    count: Int,
    onClose: () -> Unit,
    onDelete: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Surface(
        modifier = modifier.fillMaxWidth(),
        color = MaterialTheme.colorScheme.surface,
        tonalElevation = 3.dp,
        shadowElevation = 3.dp,
    ) {
        Row(
            Modifier
                .statusBarsPadding()
                .fillMaxWidth()
                .padding(horizontal = 4.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            IconButton(onClick = onClose) {
                Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Annuler")
            }
            Text(
                text = "$count sélectionné${if (count > 1) "s" else ""}",
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.weight(1f),
            )
            IconButton(onClick = onDelete) {
                Icon(Icons.Filled.Delete, contentDescription = "Supprimer")
            }
        }
    }
}

/** The floating 3-view pill plus a detached round search button. */
@Composable
private fun NavPill(
    tab: PhotosTab,
    onSelect: (PhotosTab) -> Unit,
    onSearch: () -> Unit,
    modifier: Modifier = Modifier,
) {
    Row(
        modifier = modifier.padding(horizontal = 16.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Surface(
            shape = PhotosShape.Pill,
            color = MaterialTheme.colorScheme.surface,
            tonalElevation = 3.dp,
            shadowElevation = 6.dp,
        ) {
            Row(
                Modifier.padding(horizontal = 8.dp, vertical = 6.dp),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(4.dp),
            ) {
                NavItem(PhotosTab.PHOTOS, tab, Icons.Filled.PhotoLibrary, Icons.Outlined.PhotoLibrary, "Photos", onSelect)
                NavItem(PhotosTab.COLLECTIONS, tab, Icons.Outlined.Collections, Icons.Outlined.Collections, "Albums", onSelect)
                NavItem(PhotosTab.CREATE, tab, Icons.Filled.AddCircleOutline, Icons.Filled.AddCircleOutline, "Créer", onSelect)
            }
        }
        Surface(
            shape = CircleShape,
            color = MaterialTheme.colorScheme.surface,
            tonalElevation = 3.dp,
            shadowElevation = 6.dp,
            modifier = Modifier.size(52.dp).clickable(onClick = onSearch),
        ) {
            Box(contentAlignment = Alignment.Center) {
                Icon(Icons.Filled.Search, contentDescription = "Rechercher", tint = MaterialTheme.colorScheme.onSurface)
            }
        }
    }
}

@Composable
private fun NavItem(
    self: PhotosTab,
    current: PhotosTab,
    filled: ImageVector,
    outlined: ImageVector,
    label: String,
    onSelect: (PhotosTab) -> Unit,
) {
    val active = self == current
    // The active view shows a tinted pill with its label, like the reference nav.
    Surface(
        shape = PhotosShape.Pill,
        color = if (active) PhotosColors.Blue.copy(alpha = 0.14f) else Color.Transparent,
        modifier = Modifier.clickable { onSelect(self) },
    ) {
        Row(
            Modifier.padding(horizontal = 14.dp, vertical = 8.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            Icon(
                if (active) filled else outlined,
                contentDescription = label,
                tint = if (active) PhotosColors.Blue else MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.size(22.dp),
            )
            if (active) {
                Spacer(Modifier.width(6.dp))
                Text(
                    label,
                    style = MaterialTheme.typography.labelLarge,
                    fontWeight = FontWeight.SemiBold,
                    color = PhotosColors.Blue,
                )
            }
        }
    }
}

@Composable
private fun CollectionsTabContent(albums: List<AlbumDto>) {
    Column(
        Modifier
            .fillMaxSize()
            .statusBarsPadding(),
    ) {
        Text(
            "Albums",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(16.dp),
        )
        if (albums.isEmpty()) {
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                Text("Aucun album", color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        } else {
            LazyVerticalGrid(
                columns = GridCells.Fixed(2),
                contentPadding = PaddingValues(start = 12.dp, end = 12.dp, bottom = 96.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                items(albums, key = { it.id }) { album -> AlbumCard(album) }
            }
        }
    }
}

@Composable
private fun AlbumCard(album: AlbumDto) {
    Column {
        Surface(
            shape = PhotosShape.Card,
            color = MaterialTheme.colorScheme.surfaceVariant,
            modifier = Modifier.fillMaxWidth().size(width = 0.dp, height = 150.dp),
        ) {}
        Text(
            album.name,
            style = MaterialTheme.typography.bodyMedium,
            fontWeight = FontWeight.Medium,
            modifier = Modifier.padding(top = 6.dp, start = 4.dp),
        )
        Text(
            "${album.photoCount} élément${if (album.photoCount > 1) "s" else ""}",
            style = MaterialTheme.typography.bodySmall,
            color = MaterialTheme.colorScheme.onSurfaceVariant,
            modifier = Modifier.padding(start = 4.dp),
        )
    }
}

@Composable
private fun CreateTabContent(
    onTakePhoto: () -> Unit,
    onTakeVideo: () -> Unit,
    onImport: () -> Unit,
) {
    Column(
        Modifier
            .fillMaxSize()
            .statusBarsPadding(),
    ) {
        Text(
            "Créer",
            style = MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.padding(16.dp),
        )
        CreateAction(
            icon = Icons.Filled.PhotoCamera,
            title = "Prendre une photo",
            subtitle = "Capturer et ajouter à la photothèque",
            onClick = onTakePhoto,
        )
        CreateAction(
            icon = Icons.Filled.Videocam,
            title = "Filmer une vidéo",
            subtitle = "Enregistrer et ajouter à la photothèque",
            onClick = onTakeVideo,
        )
        CreateAction(
            icon = Icons.Filled.AddPhotoAlternate,
            title = "Importer depuis l'appareil",
            subtitle = "Choisir des photos ou vidéos existantes",
            onClick = onImport,
        )
    }
}

@Composable
private fun CreateAction(
    icon: ImageVector,
    title: String,
    subtitle: String,
    onClick: () -> Unit,
) {
    Row(
        Modifier
            .fillMaxWidth()
            .clickable(onClick = onClick)
            .padding(horizontal = 16.dp, vertical = 14.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Surface(shape = CircleShape, color = PhotosColors.Blue.copy(alpha = 0.14f), modifier = Modifier.size(48.dp)) {
            Box(contentAlignment = Alignment.Center) {
                Icon(icon, contentDescription = null, tint = PhotosColors.Blue, modifier = Modifier.size(24.dp))
            }
        }
        Column(Modifier.padding(start = 16.dp)) {
            Text(title, style = MaterialTheme.typography.bodyLarge, fontWeight = FontWeight.Medium)
            Text(subtitle, style = MaterialTheme.typography.bodySmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
private fun EmptyState(message: String, onRetry: () -> Unit) {
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            modifier = Modifier.padding(32.dp),
        ) {
            Text(message, color = MaterialTheme.colorScheme.onSurfaceVariant)
            Spacer(Modifier.size(12.dp))
            Text(
                "Réessayer",
                color = PhotosColors.Blue,
                fontWeight = FontWeight.SemiBold,
                modifier = Modifier.clickable(onClick = onRetry).padding(8.dp),
            )
        }
    }
}

@Composable
private fun AccountAvatar(account: SharedAccount) {
    val initial = account.label.trim().firstOrNull()?.uppercaseChar()?.toString() ?: "?"
    Surface(
        modifier = Modifier.size(34.dp),
        shape = CircleShape,
        color = PhotosColors.Blue,
    ) {
        Box(contentAlignment = Alignment.Center) {
            Text(
                text = initial,
                color = Color.White,
                style = MaterialTheme.typography.labelLarge,
                fontWeight = FontWeight.SemiBold,
            )
        }
    }
}

@Composable
private fun NoAccount() {
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
        Column(
            horizontalAlignment = Alignment.CenterHorizontally,
            modifier = Modifier.padding(32.dp),
        ) {
            Text(
                text = "Connectez-vous depuis une application Kubuno pour voir vos photos.",
                style = MaterialTheme.typography.titleMedium,
                color = MaterialTheme.colorScheme.onSurface,
            )
        }
    }
}
