package com.kubuno.photos.net

import com.kubuno.android.account.BrokeredClients
import com.kubuno.android.account.SharedAccounts
import javax.inject.Inject
import javax.inject.Singleton
import okhttp3.Call
import okhttp3.OkHttpClient
import okhttp3.Request

/**
 * Routes a thumbnail/preview request to the shared account that owns the URL.
 *
 * The core's [com.kubuno.android.account.AccountCallFactory] serves apps that
 * OWN their accounts (it reads the private per-app registry). Photos is a pure
 * consumer, so its registry is empty; it must borrow each account's access
 * token through [BrokeredClients], exactly as its Retrofit calls do. Every
 * photo byte stream is authenticated, so an anonymous client would 401 — this
 * factory is what makes Coil load thumbnails at all.
 */
@Singleton
class PhotosCallFactory @Inject constructor(
    private val sharedAccounts: SharedAccounts,
    private val brokered: BrokeredClients,
) : Call.Factory {

    /** For URLs belonging to no known account: no credentials attached. */
    private val anonymous by lazy { OkHttpClient() }

    override fun newCall(request: Request): Call {
        val url = request.url.toString()
        val owner = sharedAccounts.list()
            // Longest prefix wins: two instances can share a host and differ
            // only by path (`https://host/` and `https://host/kubuno`).
            .filter { url.startsWith(it.serverUrl) }
            .maxByOrNull { it.serverUrl.length }
        val client = owner?.let { brokered.of(it).okHttpClient } ?: anonymous
        return client.newCall(request)
    }
}
