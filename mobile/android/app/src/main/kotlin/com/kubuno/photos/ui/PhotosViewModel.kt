package com.kubuno.photos.ui

import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.kubuno.android.account.SharedAccount
import com.kubuno.android.account.SharedAccounts
import com.kubuno.photos.net.AlbumDto
import com.kubuno.photos.net.PatchPhotoBody
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
    /** Index into [photos] of the open full-screen photo, or null when the grid is showing. */
    val viewerIndex: Int? = null,
    /** True while a captured photo/video is being uploaded. */
    val uploading: Boolean = false,
    /** One-shot user message (shown as a toast, then cleared). */
    val message: String? = null,
)

@HiltViewModel
class PhotosViewModel @Inject constructor(
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
    fun openViewer(id: String) {
        val index = _state.value.photos.indexOfFirst { it.id == id }
        if (index >= 0) _state.value = _state.value.copy(viewerIndex = index)
    }

    fun closeViewer() {
        _state.value = _state.value.copy(viewerIndex = null)
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

    fun clearMessage() {
        _state.value = _state.value.copy(message = null)
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
        val photos = _state.value.photos.map { if (it.id == id) edit(it) else it }
        _state.value = _state.value.copy(
            photos = photos,
            sections = sectionsOf(photos, _state.value.density),
        )
    }

    private fun removeLocal(ids: Set<String>) {
        val photos = _state.value.photos.filterNot { it.id in ids }
        _state.value = _state.value.copy(
            photos = photos,
            sections = sectionsOf(photos, _state.value.density),
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
