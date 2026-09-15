package com.kubuno.photos.ui

import android.content.Context
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.kubuno.android.account.SharedAccount
import dagger.hilt.android.qualifiers.ApplicationContext
import com.kubuno.android.account.SharedAccounts
import com.kubuno.photos.net.AddPhotosBody
import com.kubuno.photos.net.AlbumDto
import com.kubuno.photos.net.CreateAlbumBody
import com.kubuno.photos.net.PatchPhotoBody
import com.kubuno.photos.net.UpdateAlbumBody
import com.kubuno.photos.net.PhotoDto
import com.kubuno.photos.net.PhotosApi
import com.kubuno.photos.net.PhotosClients
import dagger.hilt.android.lifecycle.HiltViewModel
import java.time.Instant
import java.time.LocalDate
import java.time.OffsetDateTime
import java.time.ZoneId
import java.time.format.DateTimeFormatter
import java.util.Locale
import javax.inject.Inject
import okhttp3.MediaType.Companion.toMediaTypeOrNull
import okhttp3.MultipartBody
import okhttp3.RequestBody.Companion.toRequestBody
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

/** The bottom-nav destinations, mirroring the reference gallery's floating pill. */
enum class PhotosTab { PHOTOS, COLLECTIONS, CREATE }

/** One picked/captured item ready to upload: its bytes, mime type and file name. */
data class UploadItem(val bytes: ByteArray, val mime: String, val name: String)

/** One date-headed run of photos in the grid (reverse-chronological). */
data class PhotoSection(
    val key: String,
    val header: String,
    val photos: List<PhotoDto>,
)

/** Everything the gallery screen renders. */
data class PhotosUiState(
    val account: SharedAccount? = null,
    val ready: Boolean = false,
    val loading: Boolean = false,
    val error: String? = null,
    val photos: List<PhotoDto> = emptyList(),
    val sections: List<PhotoSection> = emptyList(),
    val albums: List<AlbumDto> = emptyList(),
    val density: GridDensity = GridDensity.DAY,
    val tab: PhotosTab = PhotosTab.PHOTOS,
    /** Selection mode is on whenever this is non-empty. */
    val selected: Set<String> = emptySet(),
    /** Index into [viewerPhotos] of the open full-screen photo, or null when the grid is showing. */
    val viewerIndex: Int? = null,
    /** The list the open viewer pages over (the main roll, a search, or an album). */
    val viewerPhotos: List<PhotoDto> = emptyList(),
    /** True while a captured photo/video is being uploaded. */
    val uploading: Boolean = false,
    /** One-shot user message (shown as a toast, then cleared). */
    val message: String? = null,
    // ── Search ───────────────────────────────────────────────────────────────
    val searchActive: Boolean = false,
    val query: String = "",
    /** The active browsable category: "starred", "video", "trashed", or null. */
    val activeCategory: String? = null,
    val searchLoading: Boolean = false,
    val searchResults: List<PhotoDto> = emptyList(),
    val searchSections: List<PhotoSection> = emptyList(),
    /** A downloaded file ready to hand to the system share sheet (one-shot). */
    val shareReady: ShareReady? = null,
    // ── Albums ───────────────────────────────────────────────────────────────
    /** The album currently open full-screen, or null. */
    val openAlbum: AlbumDto? = null,
    val albumLoading: Boolean = false,
    val albumPhotos: List<PhotoDto> = emptyList(),
    val albumSections: List<PhotoSection> = emptyList(),
    /** Selection within the open album (its own mode, separate from the main grid). */
    val albumSelected: Set<String> = emptySet(),
    /** Whether the "add selection to an album" sheet is showing. */
    val showAddToAlbum: Boolean = false,
    val addingToAlbum: Boolean = false,
)

/** A file downloaded for sharing, with the mime type to tag the share intent. */
data class ShareReady(val file: java.io.File, val mime: String)

@HiltViewModel
class PhotosViewModel @Inject constructor(
    @ApplicationContext private val appContext: Context,
    sharedAccounts: SharedAccounts,
    private val clients: PhotosClients,
) : ViewModel() {

    /** The shared Kubuno accounts on this device; photos is a pure consumer. */
    private val accounts: List<SharedAccount> = sharedAccounts.list()

    private val _state = MutableStateFlow(PhotosUiState(ready = false))
    val state: StateFlow<PhotosUiState> = _state.asStateFlow()

    private val api: PhotosApi? get() = _state.value.account?.let(clients::api)

    init {
        viewModelScope.launch {
            // Several apps can each register a system account for the SAME
            // identity, but only the one whose owner still holds the refresh
            // token can mint access tokens — the others resolve to a re-login
            // and would 401 every call. So pick the first account that actually
            // yields a token, rather than blindly taking the first listed.
            val account = pickUsableAccount()
            _state.value = _state.value.copy(account = account, ready = true)
            if (account != null) refresh()
        }
    }

    /** The first shared account that can currently borrow an access token. */
    private suspend fun pickUsableAccount(): SharedAccount? {
        if (accounts.size <= 1) return accounts.firstOrNull()
        val usable = withContext(Dispatchers.IO) {
            accounts.firstOrNull { account ->
                runCatching { clients.raw(account).bearer.validAccessToken() != null }
                    .getOrDefault(false)
            }
        }
        return usable ?: accounts.firstOrNull()
    }

    fun refresh() {
        val api = api ?: return
        _state.value = _state.value.copy(loading = true, error = null)
        viewModelScope.launch {
            val result = runCatching {
                withContext(Dispatchers.IO) { api.list(trashed = false, limit = 500).photos }
            }
            result.onSuccess { photos ->
                val ordered = photos.sortedByDescending { epochMillis(it) }
                _state.value = _state.value.copy(
                    loading = false,
                    photos = ordered,
                    sections = sectionsOf(ordered, _state.value.density),
                )
            }.onFailure { e ->
                _state.value = _state.value.copy(loading = false, error = e.message ?: "error")
            }
            loadAlbums()
        }
    }

    private suspend fun loadAlbums() {
        val api = api ?: return
        runCatching { withContext(Dispatchers.IO) { api.albums().albums } }
            .onSuccess { _state.value = _state.value.copy(albums = it) }
    }

    fun selectTab(tab: PhotosTab) {
        _state.value = _state.value.copy(tab = tab)
    }

    /** Pinch zoom in the grid re-columns and, at month level, re-groups by month. */
    fun setDensity(density: GridDensity) {
        if (density == _state.value.density) return
        _state.value = _state.value.copy(
            density = density,
            sections = sectionsOf(_state.value.photos, density),
        )
    }

    // ── Selection ──────────────────────────────────────────────────────────
    fun toggleSelect(id: String) {
        val cur = _state.value.selected
        _state.value = _state.value.copy(
            selected = if (id in cur) cur - id else cur + id,
        )
    }

    /** Tap on a date header selects, or clears, the whole group. */
    fun toggleSection(section: PhotoSection) {
        val ids = section.photos.map { it.id }.toSet()
        val cur = _state.value.selected
        val allSelected = cur.containsAll(ids)
        _state.value = _state.value.copy(
            selected = if (allSelected) cur - ids else cur + ids,
        )
    }

    fun clearSelection() {
        _state.value = _state.value.copy(selected = emptySet())
    }

    // ── Viewer ───────────────────────────────────────────────────────────────
    fun openViewer(id: String) = openViewerIn(_state.value.photos, id)

    fun openSearchViewer(id: String) = openViewerIn(_state.value.searchResults, id)

    fun openAlbumViewer(id: String) = openViewerIn(_state.value.albumPhotos, id)

    private fun openViewerIn(list: List<PhotoDto>, id: String) {
        val index = list.indexOfFirst { it.id == id }
        if (index >= 0) _state.value = _state.value.copy(viewerIndex = index, viewerPhotos = list)
    }

    fun closeViewer() {
        _state.value = _state.value.copy(viewerIndex = null, viewerPhotos = emptyList())
    }

    // ── Search ───────────────────────────────────────────────────────────────
    private var searchJob: Job? = null

    fun openSearch() {
        _state.value = _state.value.copy(searchActive = true)
    }

    fun closeSearch() {
        searchJob?.cancel()
        _state.value = _state.value.copy(
            searchActive = false,
            query = "",
            activeCategory = null,
            searchResults = emptyList(),
            searchSections = emptyList(),
            searchLoading = false,
        )
    }

    fun onQuery(q: String) {
        _state.value = _state.value.copy(query = q)
        searchJob?.cancel()
        searchJob = viewModelScope.launch {
            delay(300)
            runSearch()
        }
    }

    /** Toggle a browsable category (Favoris / Vidéos / Corbeille). */
    fun selectCategory(category: String?) {
        val next = if (_state.value.activeCategory == category) null else category
        _state.value = _state.value.copy(activeCategory = next)
        searchJob?.cancel()
        searchJob = viewModelScope.launch { runSearch() }
    }

    private suspend fun runSearch() {
        val api = api ?: return
        val q = _state.value.query.trim()
        val category = _state.value.activeCategory
        if (q.isEmpty() && category == null) {
            _state.value = _state.value.copy(
                searchResults = emptyList(),
                searchSections = emptyList(),
                searchLoading = false,
            )
            return
        }
        _state.value = _state.value.copy(searchLoading = true)
        val raw = runCatching {
            withContext(Dispatchers.IO) {
                api.list(
                    starred = if (category == "starred") true else null,
                    trashed = category == "trashed",
                    search = q.ifEmpty { null },
                    limit = 500,
                ).photos
            }
        }.getOrDefault(emptyList())
        // "Vidéos" has no server flag, so narrow the list by mime type here.
        val filtered = if (category == "video") raw.filter { it.isVideo } else raw
        val ordered = filtered.sortedByDescending { epochMillis(it) }
        _state.value = _state.value.copy(
            searchLoading = false,
            searchResults = ordered,
            searchSections = sectionsOf(ordered, GridDensity.DAY),
        )
    }

    // ── Capture upload ─────────────────────────────────────────────────────
    /**
     * Uploads a freshly captured photo or video, then refreshes the grid and
     * jumps to it. Bytes are read by the caller (which has the Context); the
     * view model only needs the bytes, the mime type, and a file name.
     */
    fun uploadCapture(bytes: ByteArray, mime: String, name: String) {
        val api = api ?: return
        _state.value = _state.value.copy(uploading = true, tab = PhotosTab.PHOTOS)
        viewModelScope.launch {
            val result = runCatching {
                withContext(Dispatchers.IO) {
                    val body = bytes.toRequestBody(mime.toMediaTypeOrNull())
                    val part = MultipartBody.Part.createFormData("photo", name, body)
                    api.upload(part)
                }
            }
            result.onSuccess {
                _state.value = _state.value.copy(uploading = false, message = "Ajouté à la photothèque")
                refresh()
            }.onFailure { e ->
                _state.value = _state.value.copy(uploading = false, message = "Échec de l'envoi : ${e.message ?: "erreur"}")
            }
        }
    }

    /**
     * Uploads several picked photos/videos in one go (device import), refreshing
     * once at the end. Bytes are read by the caller, which has the Context.
     */
    fun uploadBatch(items: List<UploadItem>) {
        val api = api ?: return
        if (items.isEmpty()) return
        _state.value = _state.value.copy(uploading = true, tab = PhotosTab.PHOTOS)
        viewModelScope.launch {
            val ok = withContext(Dispatchers.IO) {
                items.count { item ->
                    runCatching {
                        val body = item.bytes.toRequestBody(item.mime.toMediaTypeOrNull())
                        api.upload(MultipartBody.Part.createFormData("photo", item.name, body))
                    }.isSuccess
                }
            }
            val msg = if (ok == items.size) "$ok ajouté${plural(ok)} à la photothèque"
            else "$ok/${items.size} ajouté${plural(ok)}"
            _state.value = _state.value.copy(uploading = false, message = msg)
            refresh()
        }
    }

    private fun plural(n: Int) = if (n > 1) "s" else ""

    /**
     * Downloads a photo's original bytes to a cache file and signals the UI to
     * open the system share sheet. The bytes are authenticated through the same
     * brokered client as everything else.
     */
    fun share(photo: PhotoDto) {
        val api = api ?: return
        _state.value = _state.value.copy(message = "Préparation du partage…")
        viewModelScope.launch {
            val result = runCatching {
                withContext(Dispatchers.IO) {
                    val dir = java.io.File(appContext.cacheDir, "shared").apply { mkdirs() }
                    // A clean, human file name for the share target.
                    val name = (photo.originalName ?: photo.filename ?: "photo")
                        .replace(Regex("[\\\\/:*?\"<>|]"), "_")
                    val file = java.io.File(dir, name)
                    api.download(photo.id).byteStream().use { input ->
                        file.outputStream().use { input.copyTo(it) }
                    }
                    file
                }
            }
            result.onSuccess { file ->
                _state.value = _state.value.copy(
                    message = null,
                    shareReady = ShareReady(file, photo.mimeType ?: "*/*"),
                )
            }.onFailure {
                _state.value = _state.value.copy(message = "Partage impossible")
            }
        }
    }

    fun shareConsumed() {
        _state.value = _state.value.copy(shareReady = null)
    }

    fun clearMessage() {
        _state.value = _state.value.copy(message = null)
    }

    // ── Albums ───────────────────────────────────────────────────────────────
    /** Opens an album full-screen and loads its photos. */
    fun openAlbum(album: AlbumDto) {
        val api = api ?: return
        _state.value = _state.value.copy(
            openAlbum = album,
            albumLoading = true,
            albumPhotos = emptyList(),
            albumSections = emptyList(),
        )
        viewModelScope.launch {
            val photos = runCatching {
                withContext(Dispatchers.IO) { api.albumPhotos(album.id).photos }
            }.getOrDefault(emptyList())
            val ordered = photos.sortedByDescending { epochMillis(it) }
            _state.value = _state.value.copy(
                albumLoading = false,
                albumPhotos = ordered,
                albumSections = sectionsOf(ordered, GridDensity.DAY),
            )
        }
    }

    fun closeAlbum() {
        _state.value = _state.value.copy(
            openAlbum = null,
            albumPhotos = emptyList(),
            albumSections = emptyList(),
            albumSelected = emptySet(),
        )
    }

    // Selection inside an open album (curation: remove, set cover).
    fun toggleAlbumSelect(id: String) {
        val cur = _state.value.albumSelected
        _state.value = _state.value.copy(albumSelected = if (id in cur) cur - id else cur + id)
    }

    fun toggleAlbumSection(section: PhotoSection) {
        val ids = section.photos.map { it.id }.toSet()
        val cur = _state.value.albumSelected
        _state.value = _state.value.copy(
            albumSelected = if (cur.containsAll(ids)) cur - ids else cur + ids,
        )
    }

    fun clearAlbumSelection() {
        _state.value = _state.value.copy(albumSelected = emptySet())
    }

    /** Removes the album's selected photos from the album (not from the library). */
    fun removeSelectedFromAlbum() {
        val album = _state.value.openAlbum ?: return
        val api = api ?: return
        val ids = _state.value.albumSelected.toList()
        if (ids.isEmpty()) return
        val remaining = _state.value.albumPhotos.filterNot { it.id in ids }
        _state.value = _state.value.copy(
            albumPhotos = remaining,
            albumSections = sectionsOf(remaining, GridDensity.DAY),
            albumSelected = emptySet(),
        )
        viewModelScope.launch {
            ids.forEach { pid -> runCatching { withContext(Dispatchers.IO) { api.removeFromAlbum(album.id, pid) } } }
            loadAlbums()
        }
    }

    /** Sets the album cover to the single selected photo. */
    fun setAlbumCover(photoId: String) {
        val album = _state.value.openAlbum ?: return
        val api = api ?: return
        _state.value = _state.value.copy(albumSelected = emptySet())
        viewModelScope.launch {
            val ok = runCatching {
                withContext(Dispatchers.IO) { api.updateAlbum(album.id, UpdateAlbumBody(coverPhotoId = photoId)) }
            }.isSuccess
            loadAlbums()
            _state.value = _state.value.copy(message = if (ok) "Couverture définie" else "Échec")
        }
    }

    /** Opens the "add the current selection to an album" sheet. */
    fun promptAddToAlbum() {
        _state.value = _state.value.copy(showAddToAlbum = true)
        viewModelScope.launch { loadAlbums() }
    }

    fun dismissAddToAlbum() {
        _state.value = _state.value.copy(showAddToAlbum = false)
    }

    fun addSelectionToAlbum(albumId: String) {
        val api = api ?: return
        val ids = _state.value.selected.toList()
        if (ids.isEmpty()) return
        _state.value = _state.value.copy(addingToAlbum = true)
        viewModelScope.launch {
            val added = runCatching {
                withContext(Dispatchers.IO) { api.addToAlbum(albumId, AddPhotosBody(ids)).added }
            }.getOrNull()
            finishAddToAlbum(added)
        }
    }

    fun createAlbumWithSelection(name: String) {
        val api = api ?: return
        val trimmed = name.trim()
        if (trimmed.isEmpty()) return
        val ids = _state.value.selected.toList()
        _state.value = _state.value.copy(addingToAlbum = true)
        viewModelScope.launch {
            val added = runCatching {
                withContext(Dispatchers.IO) {
                    val album = api.createAlbum(CreateAlbumBody(trimmed)).album
                    if (ids.isNotEmpty()) {
                        val count = api.addToAlbum(album.id, AddPhotosBody(ids)).added
                        // Give the new album a cover so it isn't blank in the grid.
                        runCatching { api.updateAlbum(album.id, UpdateAlbumBody(coverPhotoId = ids.first())) }
                        count
                    } else 0
                }
            }.getOrNull()
            finishAddToAlbum(added)
        }
    }

    private suspend fun finishAddToAlbum(added: Int?) {
        loadAlbums()
        _state.value = _state.value.copy(
            addingToAlbum = false,
            showAddToAlbum = false,
            selected = if (added != null) emptySet() else _state.value.selected,
            message = if (added != null) "Ajouté à l'album" else "Échec de l'ajout à l'album",
        )
    }

    // ── Mutations ──────────────────────────────────────────────────────────
    /** Toggles the star on a photo, updating the local list optimistically. */
    fun toggleStar(id: String) {
        val api = api ?: return
        val current = _state.value.photos.firstOrNull { it.id == id } ?: return
        val next = !current.isStarred
        patchLocal(id) { it.copy(isStarred = next) }
        viewModelScope.launch {
            runCatching {
                withContext(Dispatchers.IO) { api.patch(id, PatchPhotoBody(isStarred = next)) }
            }.onFailure { patchLocal(id) { it.copy(isStarred = current.isStarred) } }
        }
    }

    /** Moves the current selection (or one photo) to the trash and drops it from the grid. */
    fun trashSelection() {
        val api = api ?: return
        val ids = _state.value.selected.toList()
        if (ids.isEmpty()) return
        removeLocal(ids.toSet())
        clearSelection()
        viewModelScope.launch {
            ids.forEach { id ->
                runCatching { withContext(Dispatchers.IO) { api.trash(id) } }
            }
        }
    }

    private fun patchLocal(id: String, edit: (PhotoDto) -> PhotoDto) {
        val map = { list: List<PhotoDto> -> list.map { if (it.id == id) edit(it) else it } }
        val photos = map(_state.value.photos)
        val results = map(_state.value.searchResults)
        val album = map(_state.value.albumPhotos)
        _state.value = _state.value.copy(
            photos = photos,
            sections = sectionsOf(photos, _state.value.density),
            searchResults = results,
            searchSections = sectionsOf(results, GridDensity.DAY),
            albumPhotos = album,
            albumSections = sectionsOf(album, GridDensity.DAY),
            viewerPhotos = map(_state.value.viewerPhotos),
        )
    }

    private fun removeLocal(ids: Set<String>) {
        val drop = { list: List<PhotoDto> -> list.filterNot { it.id in ids } }
        val photos = drop(_state.value.photos)
        val results = drop(_state.value.searchResults)
        val album = drop(_state.value.albumPhotos)
        _state.value = _state.value.copy(
            photos = photos,
            sections = sectionsOf(photos, _state.value.density),
            searchResults = results,
            searchSections = sectionsOf(results, GridDensity.DAY),
            albumPhotos = album,
            albumSections = sectionsOf(album, GridDensity.DAY),
            viewerPhotos = drop(_state.value.viewerPhotos),
        )
    }

    // ── Date grouping ──────────────────────────────────────────────────────
    private fun sectionsOf(photos: List<PhotoDto>, density: GridDensity): List<PhotoSection> {
        val byMonth = density == GridDensity.MONTH
        val today = LocalDate.now(ZONE)
        val yesterday = today.minusDays(1)
        // LinkedHashMap keeps the reverse-chronological order the list already has.
        val groups = LinkedHashMap<String, MutableList<PhotoDto>>()
        val headers = HashMap<String, String>()
        for (p in photos) {
            val date = localDate(p)
            val key: String
            val header: String
            if (byMonth) {
                key = date?.let { "%04d-%02d".format(it.year, it.monthValue) } ?: "unknown"
                header = date?.format(MONTH_FMT)?.replaceFirstChar { it.uppercase() } ?: UNKNOWN
            } else {
                key = date?.toString() ?: "unknown"
                header = when (date) {
                    null -> UNKNOWN
                    today -> "Aujourd'hui"
                    yesterday -> "Hier"
                    else -> date.format(DAY_FMT).replaceFirstChar { it.uppercase() }
                }
            }
            groups.getOrPut(key) { mutableListOf() }.add(p)
            headers[key] = header
        }
        return groups.map { (key, list) -> PhotoSection(key, headers[key] ?: UNKNOWN, list) }
    }

    private fun localDate(photo: PhotoDto): LocalDate? =
        epochMillis(photo)?.let { Instant.ofEpochMilli(it).atZone(ZONE).toLocalDate() }

    private fun epochMillis(photo: PhotoDto): Long? {
        val iso = photo.instant ?: return null
        return runCatching { OffsetDateTime.parse(iso).toInstant().toEpochMilli() }
            .recoverCatching { Instant.parse(iso).toEpochMilli() }
            .recoverCatching {
                LocalDate.parse(iso.take(10)).atStartOfDay(ZONE).toInstant().toEpochMilli()
            }
            .getOrNull()
    }

    private companion object {
        val ZONE: ZoneId = ZoneId.systemDefault()
        val DAY_FMT: DateTimeFormatter = DateTimeFormatter.ofPattern("EEEE d MMMM", Locale.FRENCH)
        val MONTH_FMT: DateTimeFormatter = DateTimeFormatter.ofPattern("MMMM yyyy", Locale.FRENCH)
        const val UNKNOWN = "Sans date"
    }
}
