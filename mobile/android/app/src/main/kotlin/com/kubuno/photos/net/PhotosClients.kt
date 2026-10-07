package com.kubuno.photos.net

import com.kubuno.android.account.BrokeredClient
import com.kubuno.android.account.BrokeredClients
import com.kubuno.android.account.SharedAccount
import java.util.concurrent.ConcurrentHashMap
import javax.inject.Inject
import javax.inject.Singleton
import kotlinx.serialization.json.Json
import okhttp3.MediaType.Companion.toMediaType
import retrofit2.Retrofit
import retrofit2.converter.kotlinx.serialization.asConverterFactory

/**
 * Builds a [PhotosApi] per shared account, over the account's borrowed-token
 * client. Like mail and maps, photos is a consumer: it never holds a refresh
 * token, it talks to each instance through the client :core-account brokers.
 */
@Singleton
class PhotosClients @Inject constructor(
    private val brokered: BrokeredClients,
) {
    private val json = Json {
        ignoreUnknownKeys = true
        explicitNulls = false
    }
    private val apis = ConcurrentHashMap<String, PhotosApi>()

    fun api(account: SharedAccount): PhotosApi = apis.computeIfAbsent(account.systemName) {
        val client = brokered.of(account)
        Retrofit.Builder()
            .baseUrl(client.serverUrl + "/")
            .client(client.okHttpClient)
            .addConverterFactory(json.asConverterFactory("application/json".toMediaType()))
            .build()
            .create(PhotosApi::class.java)
    }

    /** The authenticated client itself, for photo byte streams. */
    fun raw(account: SharedAccount): BrokeredClient = brokered.of(account)
}
