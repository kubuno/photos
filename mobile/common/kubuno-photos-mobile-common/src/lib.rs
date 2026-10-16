//! Kubuno Photos for mobile - the app, portable (no OS, no window system).
//!
//! The app is a set of `.kbview` screens (`views/`, edited in Visual Studio like every Kubuno view) rendered by the
//! shared Kubuno mobile renderer (`kubuno-mobile-render`, core repository). The Android and iOS entry points
//! (`../android`, `../ios`) hand [`APP`] to the shared shell of their OS.
//!
//! Migration status: the screens are converted; the app's behaviour (its data and server calls) is still to be
//! migrated from the reference Kotlin app that lived in `mobile/` before this layout (kept in the git history).

use kubuno_mobile_render::App;

/// The `gallery.kbview` screen.
pub const GALLERY: &str = include_str!("views/gallery.kbview");
/// The `album.kbview` screen.
pub const ALBUM: &str = include_str!("views/album.kbview");
/// The `albums.kbview` screen.
pub const ALBUMS: &str = include_str!("views/albums.kbview");
/// The `create.kbview` screen.
pub const CREATE: &str = include_str!("views/create.kbview");
/// The `onboarding.kbview` screen.
pub const ONBOARDING: &str = include_str!("views/onboarding.kbview");
/// The `search.kbview` screen.
pub const SEARCH: &str = include_str!("views/search.kbview");
/// The `viewer.kbview` screen.
pub const VIEWER: &str = include_str!("views/viewer.kbview");

/// Every screen, in a tap-to-advance order (the home screen first).
pub const VIEWS: &[&str] = &[GALLERY, ALBUM, ALBUMS, CREATE, ONBOARDING, SEARCH, VIEWER];

/// The sample photos the screens show (`Image="…"`), embedded JPEGs (see `assets/photos/README.md`).
const IMAGES: &[(&str, &[u8])] = &[
    ("hero1", include_bytes!("assets/photos/hero1.jpg")),
    ("hero2", include_bytes!("assets/photos/hero2.jpg")),
    ("p01", include_bytes!("assets/photos/p01.jpg")),
    ("p02", include_bytes!("assets/photos/p02.jpg")),
    ("p03", include_bytes!("assets/photos/p03.jpg")),
    ("p04", include_bytes!("assets/photos/p04.jpg")),
    ("p05", include_bytes!("assets/photos/p05.jpg")),
    ("p06", include_bytes!("assets/photos/p06.jpg")),
    ("p07", include_bytes!("assets/photos/p07.jpg")),
    ("p08", include_bytes!("assets/photos/p08.jpg")),
    ("p09", include_bytes!("assets/photos/p09.jpg")),
    ("p10", include_bytes!("assets/photos/p10.jpg")),
    ("p11", include_bytes!("assets/photos/p11.jpg")),
    ("p12", include_bytes!("assets/photos/p12.jpg")),
];

/// Register what the app's screens need from the renderer: their images. The shell calls it once, on its UI thread,
/// before the first frame (through [`APP`]); a tool rendering the views off-device calls it first too.
pub fn setup() {
    for (key, jpeg) in IMAGES {
        kubuno_mobile_render::register_image(key, jpeg);
    }
}

/// The Kubuno Photos app, as the shells run it.
pub const APP: App = App { id: "photos", label: "Kubuno Photos", views: VIEWS, setup, on_event: None, view_index: None };

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_screen_renders() {
        setup();
        for src in VIEWS {
            assert!(kubuno_mobile_render::render_view(src, 1080, 2340, None, None).is_some());
        }
    }

    #[test]
    fn every_image_decodes() {
        for (key, jpeg) in IMAGES {
            assert!(kubuno_mobile_render::decode_jpeg(jpeg).is_some(), "{key} decodes");
        }
    }
}
