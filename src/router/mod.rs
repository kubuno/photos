use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{delete, get, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    handlers::{albums, health, photos, public, shares, transform},
    middleware::require_auth,
    state::AppState,
};

pub fn build(state: AppState) -> Router {
    let authed = Router::new()
        // Photos
        .route("/",                       get(photos::list).post(photos::upload)
                                              .layer(DefaultBodyLimit::max(state.settings.photos.max_upload_bytes as usize)))
        .route("/:id",                    get(photos::get).patch(photos::update).delete(photos::delete))
        .route("/:id/download",           get(photos::download))
        .route("/:id/thumbnail",          get(photos::thumbnail))
        .route("/:id/preview",            get(photos::preview))
        .route("/:id/trash",              post(photos::trash))
        .route("/:id/restore",            post(photos::restore))
        .route("/:id/transform",          post(transform::transform))
        // Albums
        .route("/albums",                 get(albums::list).post(albums::create))
        .route("/albums/:id",             get(albums::get).patch(albums::update).delete(albums::delete))
        .route("/albums/:id/photos",      get(albums::list_photos).post(albums::add_photos))
        .route("/albums/:id/photos/:pid", delete(albums::remove_photo))
        // Partages
        .route("/shares",                 get(shares::list).post(shares::create))
        .route("/shares/:id",             delete(shares::revoke))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .with_state(state.clone());

    let public_routes = Router::new()
        .route("/share/:token",          get(public::info))
        .route("/share/:token/download", get(public::download))
        .with_state(state.clone());

    let system = Router::new()
        .route("/health", get(health::health))
        .with_state(state);

    Router::new()
        .merge(system)
        .merge(public_routes)
        .nest("/", authed)
        .layer(DefaultBodyLimit::disable())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
