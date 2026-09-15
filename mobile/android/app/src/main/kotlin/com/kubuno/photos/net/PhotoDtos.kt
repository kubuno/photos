package com.kubuno.photos.net

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable

/**
 * A photo as the module returns it. Only the fields the app renders are mapped;
 * unknown keys are ignored by the client's lenient Json. Bytes (thumbnail,
 * preview, original) are never in here — they are separate authenticated GETs
 * under /api/v1/photos/<id>/{thumbnail,preview,download}.
 */
@Serializable
data class PhotoDto(
    val id: String,
    val filename: String? = null,
    @SerialName("original_name") val originalName: String? = null,
    @SerialName("mime_type") val mimeType: String? = null,
    @SerialName("size_bytes") val sizeBytes: Long = 0,
    val width: Int? = null,
    val height: Int? = null,
    @SerialName("taken_at") val takenAt: String? = null,
    @SerialName("created_at") val createdAt: String? = null,
    @SerialName("camera_make") val cameraMake: String? = null,
    @SerialName("camera_model") val cameraModel: String? = null,
    @SerialName("gps_lat") val gpsLat: Double? = null,
    @SerialName("gps_lon") val gpsLon: Double? = null,
    @SerialName("has_thumbnail") val hasThumbnail: Boolean = true,
    @SerialName("has_preview") val hasPreview: Boolean = true,
    @SerialName("is_starred") val isStarred: Boolean = false,
    @SerialName("is_trashed") val isTrashed: Boolean = false,
    val description: String? = null,
) {
    /** Best available capture instant, in ISO-8601; falls back to import time. */
    val instant: String? get() = takenAt ?: createdAt

    /** True when this item is a video (drives the play badge on the tile). */
    val isVideo: Boolean get() = mimeType?.startsWith("video/") == true
}

/** The list envelope: `GET /` returns `{ "photos": [...] }`. */
@Serializable
data class PhotoListResponse(
    val photos: List<PhotoDto> = emptyList(),
)

/** A single-photo envelope: `GET /:id`, `PATCH /:id` return `{ "photo": {...} }`. */
@Serializable
data class PhotoResponse(
    val photo: PhotoDto,
)

/** An album as the module returns it. */
@Serializable
data class AlbumDto(
    val id: String,
    val name: String,
    val description: String? = null,
    @SerialName("cover_photo_id") val coverPhotoId: String? = null,
    @SerialName("photo_count") val photoCount: Int = 0,
    @SerialName("is_shared") val isShared: Boolean = false,
)

/** The album list envelope: `GET /albums` returns `{ "albums": [...] }`. */
@Serializable
data class AlbumListResponse(
    val albums: List<AlbumDto> = emptyList(),
)

/** A single-album envelope: `POST /albums`, `GET /albums/:id` return `{ "album": {...} }`. */
@Serializable
data class AlbumResponse(
    val album: AlbumDto,
)

/** Body for `POST /albums`. */
@Serializable
data class CreateAlbumBody(
    val name: String,
    val description: String? = null,
)

/** Body for `POST /albums/:id/photos`. */
@Serializable
data class AddPhotosBody(
    @SerialName("photo_ids") val photoIds: List<String>,
)

/** `POST /albums/:id/photos` returns `{ "added": n }`. */
@Serializable
data class AddedResponse(
    val added: Int = 0,
)

/** Body for `PATCH /:id` — only the fields the app edits. */
@Serializable
data class PatchPhotoBody(
    @SerialName("is_starred") val isStarred: Boolean? = null,
    val description: String? = null,
)
