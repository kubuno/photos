//! Instance-wide settings of the photos module, as the administrator left them in
//! the console.
//!
//! Declared by `module.toml`'s `[[settings]]`, stored in `core.settings`, and read
//! back here through `/internal/modules/photos/settings` — a module owns its own
//! schema and cannot read the core's tables, and a background worker has no user
//! token for the public config route. The module is named in the URL so the read
//! works whether the instance shares one master secret or a derived one per
//! module.
//!
//! Every field here is read by code that acts on it: a knob that changes nothing
//! is worse than an absent one.

use serde_json::Value;

#[derive(Debug, Clone, Copy)]
pub struct InstanceConfig {
    /// Edge, in pixels, of a generated square thumbnail.
    pub thumbnail_size: u32,
    /// Longest edge, in pixels, of the generated full-screen preview.
    pub preview_size: u32,
    /// JPEG quality (1–100) of generated thumbnails and previews.
    pub jpeg_quality: u8,
    /// Days a trashed photo is kept before the cleaner purges it. `0` = never.
    pub trash_auto_delete_days: i32,
    /// Whether users may create public share links at all.
    pub allow_public_sharing: bool,
    /// Ceiling on a public link's lifetime, in days. `0` = no ceiling. A link
    /// with no expiry, or one asking for longer, is clamped to this many days.
    pub share_link_max_days: i64,
    /// Largest accepted import, in megabytes. Enforced before anything is
    /// written to storage.
    pub max_upload_mb: i64,
    /// Whether formats this build cannot decode (HEIC/HEIF/AVIF) may still be
    /// imported. They are stored verbatim but get no dimensions and no
    /// derivatives, so an instance that wants a consistent gallery refuses them.
    pub accept_undecodable_formats: bool,
    /// Whether a public link may hand over the ORIGINAL file. Off, the link
    /// still shows the picture — it serves the downscaled preview instead.
    pub share_allow_download: bool,
    /// Whether a public link discloses capture metadata: date taken, camera and
    /// — the sensitive one — GPS coordinates. Off by default: a link shared with
    /// the outside must not leak where somebody lives.
    pub share_expose_metadata: bool,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            thumbnail_size:             256,
            preview_size:               1200,
            jpeg_quality:               85,
            trash_auto_delete_days:     30,
            allow_public_sharing:       true,
            share_link_max_days:        30,
            max_upload_mb:              100,
            accept_undecodable_formats: true,
            share_allow_download:       true,
            share_expose_metadata:      false,
        }
    }
}

/// Hard ceiling the request body limit is sized against, so raising
/// `max_upload_mb` in the console never runs into a lower limit frozen into the
/// router at boot. See `router::build`.
pub const MAX_UPLOAD_MB_CEILING: i64 = 500;

impl InstanceConfig {
    /// Maps the core's `{key: value}` object onto the struct. Every read falls
    /// back to the compiled default rather than to a permissive value; an
    /// out-of-range number is treated as a mistake and ignored the same way.
    /// `0` is a MEANINGFUL value for the two "days" settings (never / unlimited),
    /// so it is accepted there rather than floored away.
    pub fn from_settings(settings: &Value) -> Self {
        let d = Self::default();
        let int_in = |key: &str, min: i64, max: i64, fallback: i64| -> i64 {
            settings
                .get(key)
                .and_then(Value::as_i64)
                .filter(|n| (min..=max).contains(n))
                .unwrap_or(fallback)
        };
        let bool_of = |key: &str, fallback: bool| {
            settings.get(key).and_then(Value::as_bool).unwrap_or(fallback)
        };
        // `accepted_formats` is an enum of two strings rather than a boolean: the
        // console then reads "les formats décodables" / "tous les formats", which
        // says what happens, where a checkbox labelled "undecodable" would not.
        let accept_undecodable = settings
            .get("accepted_formats")
            .and_then(Value::as_str)
            .map(|v| v != "decodable")
            .unwrap_or(d.accept_undecodable_formats);

        Self {
            thumbnail_size:             int_in("thumbnail_size", 32, 2048, d.thumbnail_size as i64) as u32,
            preview_size:               int_in("preview_size", 256, 8192, d.preview_size as i64) as u32,
            jpeg_quality:               int_in("jpeg_quality", 1, 100, d.jpeg_quality as i64) as u8,
            trash_auto_delete_days:     int_in("trash_auto_delete_days", 0, 3650, d.trash_auto_delete_days as i64) as i32,
            allow_public_sharing:       bool_of("allow_public_sharing", d.allow_public_sharing),
            share_link_max_days:        int_in("share_link_max_days", 0, 3650, d.share_link_max_days),
            max_upload_mb:              int_in("max_upload_mb", 1, MAX_UPLOAD_MB_CEILING, d.max_upload_mb),
            accept_undecodable_formats: accept_undecodable,
            share_allow_download:       bool_of("share_allow_download", d.share_allow_download),
            share_expose_metadata:      bool_of("share_expose_metadata", d.share_expose_metadata),
        }
    }

    /// The upload ceiling in bytes, as the import path checks it.
    pub fn max_upload_bytes(&self) -> u64 {
        (self.max_upload_mb.max(1) as u64).saturating_mul(1024 * 1024)
    }
}

/// Reads the instance settings from the core. Any failure yields `None`, so the
/// caller keeps the values it already had rather than reverting to defaults
/// because the core was briefly unreachable.
pub async fn fetch(http: &reqwest::Client, core_url: &str, secret: &str) -> Option<InstanceConfig> {
    let url = format!("{core_url}/internal/modules/photos/settings");
    let resp = http
        .get(&url)
        .header("X-Internal-Secret", secret)
        .send()
        .await
        .map_err(|e| tracing::warn!(error = %e, "Lecture des réglages d'instance photos"))
        .ok()?;

    if !resp.status().is_success() {
        tracing::warn!(status = %resp.status(), "Réglages d'instance photos refusés par le core");
        return None;
    }

    let body: Value = resp
        .json()
        .await
        .map_err(|e| tracing::warn!(error = %e, "Réglages d'instance photos : réponse illisible"))
        .ok()?;

    Some(InstanceConfig::from_settings(body.get("settings")?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn missing_keys_keep_the_compiled_defaults() {
        let c = InstanceConfig::from_settings(&json!({}));
        assert_eq!(c.thumbnail_size, 256);
        assert_eq!(c.jpeg_quality, 85);
        assert_eq!(c.trash_auto_delete_days, 30);
        assert!(c.allow_public_sharing);
        assert_eq!(c.share_link_max_days, 30);
    }

    #[test]
    fn zero_is_meaningful_for_days_settings() {
        let c = InstanceConfig::from_settings(&json!({
            "trash_auto_delete_days": 0, "share_link_max_days": 0,
        }));
        assert_eq!(c.trash_auto_delete_days, 0); // never purge
        assert_eq!(c.share_link_max_days, 0);    // no expiry ceiling
    }

    #[test]
    fn out_of_range_quality_falls_back() {
        let c = InstanceConfig::from_settings(&json!({ "jpeg_quality": 500 }));
        assert_eq!(c.jpeg_quality, 85);
    }

    /// The two settings a public link's privacy hangs on default to the
    /// CAUTIOUS value the module shipped with: capture metadata (GPS included)
    /// is not disclosed unless an administrator asks for it.
    #[test]
    fn share_metadata_is_withheld_by_default() {
        let c = InstanceConfig::from_settings(&json!({}));
        assert!(!c.share_expose_metadata);
        assert!(c.share_allow_download);
    }

    #[test]
    fn upload_ceiling_is_expressed_in_bytes_and_bounded() {
        let c = InstanceConfig::from_settings(&json!({ "max_upload_mb": 25 }));
        assert_eq!(c.max_upload_bytes(), 25 * 1024 * 1024);
        // Beyond the ceiling the router was sized against, the value is a
        // mistake and the default stands rather than a limit nothing enforces.
        let c = InstanceConfig::from_settings(&json!({ "max_upload_mb": 100_000 }));
        assert_eq!(c.max_upload_mb, 100);
    }

    #[test]
    fn only_the_decodable_value_narrows_the_accepted_formats() {
        let strict = InstanceConfig::from_settings(&json!({ "accepted_formats": "decodable" }));
        assert!(!strict.accept_undecodable_formats);
        let all = InstanceConfig::from_settings(&json!({ "accepted_formats": "all" }));
        assert!(all.accept_undecodable_formats);
        // An unknown value must not silently tighten an import policy.
        let junk = InstanceConfig::from_settings(&json!({ "accepted_formats": "nawak" }));
        assert!(junk.accept_undecodable_formats);
    }
}
