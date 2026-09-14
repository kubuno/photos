package com.kubuno.photos.ui

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.kubuno.android.account.SharedAccount

/**
 * Photos-only design language — a flat, edge-to-edge gallery with a floating
 * "pill" navigation, sitting on top of the shared KubunoTheme WITHOUT changing
 * it. The signature is the four-petal pinwheel accent (like the reference
 * gallery apps): a small burst of colour on selection states and the memories
 * carousel, over an otherwise very plain, image-first surface.
 */
object PhotosColors {
    // Brand accent, mirroring the web module (photos/frontend/src/theme.css).
    val Blue = Color(0xFF1A73E8)

    // The four-petal pinwheel: the colour signature reused for selection rings,
    // memory-card tints and the accent dots.
    val PetalRed = Color(0xFFEA4335)
    val PetalYellow = Color(0xFFFBBC04)
    val PetalGreen = Color(0xFF34A853)
    val PetalBlue = Color(0xFF4285F4)

    val Petals = listOf(PetalRed, PetalYellow, PetalGreen, PetalBlue)

    /** A stable pinwheel colour for a key (e.g. a memory card), so it is steady across recompositions. */
    fun petalFor(key: String): Color = Petals[(key.hashCode() and Int.MAX_VALUE) % Petals.size]
}

/** Rounded corners used across the gallery. */
object PhotosShape {
    val Pill = RoundedCornerShape(50)
    val Card = RoundedCornerShape(20.dp)
    val Memory = RoundedCornerShape(18.dp)
}

/** The three pinch-zoom density levels, mirroring the reference gallery. */
enum class GridDensity(val columns: Int) {
    COMFORTABLE(2),
    DAY(4),
    MONTH(5),
    ;

    fun zoomIn(): GridDensity = when (this) {
        MONTH -> DAY
        DAY -> COMFORTABLE
        COMFORTABLE -> COMFORTABLE
    }

    fun zoomOut(): GridDensity = when (this) {
        COMFORTABLE -> DAY
        DAY -> MONTH
        MONTH -> MONTH
    }
}

/**
 * Absolute URL of a photo's thumbnail on its instance. Coil authenticates the
 * GET through [com.kubuno.photos.net.PhotosCallFactory], which matches this URL
 * to [account] by prefix.
 */
fun thumbUrl(account: SharedAccount, photoId: String): String =
    "${account.serverUrl.trimEnd('/')}/api/v1/photos/$photoId/thumbnail"

/** Absolute URL of a photo's full-screen preview. */
fun previewUrl(account: SharedAccount, photoId: String): String =
    "${account.serverUrl.trimEnd('/')}/api/v1/photos/$photoId/preview"
