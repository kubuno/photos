package com.kubuno.photos

import android.app.Application
import coil3.ImageLoader
import coil3.PlatformContext
import coil3.SingletonImageLoader
import coil3.disk.DiskCache
import coil3.disk.directory
import coil3.memory.MemoryCache
import coil3.network.okhttp.OkHttpNetworkFetcherFactory
import coil3.request.crossfade
import com.kubuno.android.account.AccountManagerBridge
import com.kubuno.photos.net.PhotosCallFactory
import dagger.hilt.android.HiltAndroidApp
import javax.inject.Inject

@HiltAndroidApp
class KubunoPhotosApp : Application(), SingletonImageLoader.Factory {

    @Inject lateinit var accountBridge: AccountManagerBridge
    @Inject lateinit var callFactory: PhotosCallFactory

    override fun onCreate() {
        super.onCreate()
        // Keep the system accounts in step with ours, exactly like the drive,
        // mail and maps apps: whichever app the user opens keeps the shared list
        // current.
        accountBridge.install()
    }

    /**
     * A gallery scrolls hundreds of thumbnails, so give Coil a generous memory
     * and disk cache. Each request rides the account-authenticated client the
     * [PhotosCallFactory] picks by URL — thumbnails are never public bytes.
     */
    override fun newImageLoader(context: PlatformContext): ImageLoader =
        ImageLoader.Builder(context)
            .components {
                add(OkHttpNetworkFetcherFactory(callFactory = { callFactory }))
            }
            .memoryCache {
                MemoryCache.Builder()
                    .maxSizePercent(context, 0.25)
                    .build()
            }
            .diskCache {
                DiskCache.Builder()
                    .directory(cacheDir.resolve("photo_thumbs"))
                    .maxSizeBytes(256L * 1024 * 1024)
                    .build()
            }
            .crossfade(true)
            .build()
}
