package com.kubuno.photos.net

import okhttp3.MultipartBody
import okhttp3.ResponseBody
import retrofit2.http.Body
import retrofit2.http.Streaming
import retrofit2.http.DELETE
import retrofit2.http.GET
import retrofit2.http.Multipart
import retrofit2.http.PATCH
import retrofit2.http.POST
import retrofit2.http.Part
import retrofit2.http.Path
import retrofit2.http.Query

/**
 * The photos module REST surface, proxied by the core at /api/v1/photos.
 *
 * Byte streams (thumbnail/preview/download) are NOT here: Coil fetches those
 * directly through the account-authenticated call factory, so they never pass
 * through Retrofit.
 */
interface PhotosApi {

    @GET("api/v1/photos/")
    suspend fun list(
        @Query("album_id") albumId: String? = null,
        @Query("starred") starred: Boolean? = null,
        @Query("trashed") trashed: Boolean? = null,
        @Query("search") search: String? = null,
        @Query("limit") limit: Int? = null,
        @Query("offset") offset: Int? = null,
    ): PhotoListResponse

    /** Uploads one captured photo/video. The module reads the "photo" part. */
    @Multipart
    @POST("api/v1/photos/")
    suspend fun upload(@Part photo: MultipartBody.Part): PhotoResponse

    /** The original bytes, for sharing/exporting. */
    @Streaming
    @GET("api/v1/photos/{id}/download")
    suspend fun download(@Path("id") id: String): ResponseBody

    @GET("api/v1/photos/{id}")
    suspend fun get(@Path("id") id: String): PhotoResponse

    @PATCH("api/v1/photos/{id}")
    suspend fun patch(@Path("id") id: String, @Body body: PatchPhotoBody): PhotoResponse

    @POST("api/v1/photos/{id}/trash")
    suspend fun trash(@Path("id") id: String)

    @POST("api/v1/photos/{id}/restore")
    suspend fun restore(@Path("id") id: String)

    @DELETE("api/v1/photos/{id}")
    suspend fun delete(@Path("id") id: String)

    @GET("api/v1/photos/albums")
    suspend fun albums(): AlbumListResponse

    @POST("api/v1/photos/albums")
    suspend fun createAlbum(@Body body: CreateAlbumBody): AlbumResponse

    @GET("api/v1/photos/albums/{id}/photos")
    suspend fun albumPhotos(@Path("id") id: String): PhotoListResponse

    @POST("api/v1/photos/albums/{id}/photos")
    suspend fun addToAlbum(@Path("id") id: String, @Body body: AddPhotosBody): AddedResponse

    @PATCH("api/v1/photos/albums/{id}")
    suspend fun updateAlbum(@Path("id") id: String, @Body body: UpdateAlbumBody): AlbumResponse

    @DELETE("api/v1/photos/albums/{id}/photos/{pid}")
    suspend fun removeFromAlbum(@Path("id") id: String, @Path("pid") pid: String)
}
