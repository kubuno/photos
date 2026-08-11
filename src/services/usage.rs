//! Declaring to the core what this module stores, per account.
//!
//! ## Attribution
//!
//! Whoever physically holds the byte declares it, and only them. Photos is the
//! ordinary case: originals and derivatives are written straight to its own
//! `StorageBackend`, nothing is delegated to another module, so photos declares
//! everything it stores and nobody else does.
//!
//! ## The three categories
//!
//! The core bills what a person can free themselves. That rule decides all three
//! lines below.
//!
//! * `content` — live photos (`is_trashed = FALSE`). Billed: deleting a photo
//!   releases it.
//! * `trash` — the same originals after a soft delete. **Also billed, and kept as
//!   its own line rather than folded into `content`.** Billed because emptying the
//!   bin is exactly the thing the person can do to free the space; separate
//!   because when an account saturates, "half of it is in your bin" is the single
//!   most useful thing the console can say. Only [`super::photo_service::delete_photo`]
//!   actually releases the bytes, and only then does the figure fall.
//! * `thumbnails` — the derived JPEGs (`photos/{owner}/thumbs/…` and
//!   `photos/{owner}/previews/…`). Real bytes, but the person has no handle on
//!   them — no route deletes a thumbnail, and photos regenerates them anyway — so
//!   they are not billed. Counted for both live and trashed photos: the files
//!   survive a soft delete exactly like the original.
//!
//! Nothing else. No `versions` and no `retention`: photos keeps no history at all
//! — a transform overwrites the original in place, so there is no prior state
//! either the person or the module is holding on to. No `cache`, no `index` (EXIF
//! lives in columns of the photos row, metadata rather than stored payload).
//!
//! ## State, never deltas
//!
//! Every declaration carries the module's **current** figures for the accounts
//! it names, so re-sending one changes nothing and a lost message costs one stale
//! number until the next declaration repairs it. The core keys rows on
//! `(module_id, user_id, category)`; idempotence is structural.
//!
//! ## Two rhythms
//!
//! * **Incremental.** Upload, trash, restore, hard delete and transform each call
//!   [`mark_dirty`]. A background task coalesces the marks and declares a few
//!   seconds later, so importing four hundred photos produces one declaration
//!   rather than four hundred, and the import never waits on the core. Photos
//!   earns this — unlike a module holding only slow-moving machinery, its figures
//!   are the ones a person watches move while they upload.
//! * **Full.** At startup and every six hours, every account is recounted and
//!   declared as complete state. This is the repair path: it corrects whatever the
//!   incremental path missed while the core was unreachable, and it is the only
//!   declaration that can retire an account photos no longer holds anything for.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;
use std::time::Duration;

use serde_json::json;
use sqlx::PgPool;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::state::AppState;

/// How long marks are coalesced before a declaration is sent. Long enough that a
/// bulk import collapses into one call, short enough that the console is right by
/// the time somebody switches to it.
const FLUSH_INTERVAL: Duration = Duration::from_secs(5);

/// How often the complete state is recounted and declared.
const FULL_SYNC_INTERVAL: Duration = Duration::from_secs(6 * 3_600);

/// First retry delay when a full declaration could not be delivered, doubling up
/// to [`FULL_SYNC_INTERVAL`].
///
/// The module starts before the core has necessarily finished accepting
/// registrations, so the very first full declaration routinely fails. Without a
/// backoff it would be re-attempted six hours later and the breakdown would sit
/// empty for an afternoon after every reboot.
const FULL_RETRY_MIN: Duration = Duration::from_secs(15);

/// Matches the core's own per-request ceiling (`storage::usage::MAX_ENTRIES`).
/// A larger instance is declared in several calls.
const MAX_ENTRIES: usize = 5_000;

/// Rows measured against the storage backend per backfill batch.
const BACKFILL_BATCH: i64 = 200;

/// Batches per full sync. The bound matters: measuring a derivative costs one
/// `size()` round-trip against the backend (a `stat` locally, an HTTP call on
/// S3), and a library imported before `derived_bytes` existed could hold hundreds
/// of thousands of rows. 20 × 200 = 4 000 photos (8 000 stats) per sync converges
/// a normal library on the first pass and a very large one within a day, without
/// ever letting the reporter turn into a storm against the object store.
const BACKFILL_MAX_BATCHES: usize = 20;

/// Identifier this module declares under. Only consulted by the core when the
/// caller could not be identified from its `X-Internal-Secret`: the core prefers
/// the secret's identity and answers 403 when the two disagree, so naming
/// ourselves in the body can never impersonate another module. It is what makes
/// the declaration work while the module spawner still hands out the master
/// secret, and becomes dead weight — not a bypass — once it derives per-module
/// secrets. `/internal/modules/register` already works this way.
const MODULE_ID: &str = "photos";

/// Live originals — billed: the person frees them by deleting a photo.
const CAT_CONTENT: &str = "content";
/// Soft-deleted originals — still on disk, still billed: the person frees them by
/// emptying the bin. Never merged into `content`.
const CAT_TRASH: &str = "trash";
/// Derived images — not billed: nothing exposes them for the person to delete.
const CAT_THUMBNAILS: &str = "thumbnails";

/// Marks left by the write paths, drained by the reporter task.
///
/// A global rather than a field on `AppState` because the write paths are free
/// functions in `photo_service` reached with nothing but a `PgPool` — threading a
/// channel through all of them would touch every one of them to deliver a
/// notification.
static DIRTY: OnceLock<mpsc::UnboundedSender<Uuid>> = OnceLock::new();

/// Signals that an account's photos total has changed.
///
/// Never blocks and never fails the caller's work: a dropped mark costs one stale
/// figure until the next full sync, which is not worth failing an upload over.
/// Before the reporter task has started this is a no-op.
pub fn mark_dirty(owner_id: Uuid) {
    if let Some(tx) = DIRTY.get() {
        let _ = tx.send(owner_id);
    }
}

/// One `(account, category)` figure, as declared.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    user_id: Uuid,
    category: &'static str,
    used_bytes: i64,
    object_count: i64,
}

/// The three per-account aggregates, in one pass over the photos table.
///
/// `derived_bytes` is summed for **all** photos, trashed or not: a soft delete
/// leaves the thumbnail and the preview exactly where they are, and the object
/// count is the number of derivative *files*, which is why the two boolean
/// filters are added rather than counting rows.
struct Totals {
    content_bytes: i64,
    content_count: i64,
    trash_bytes: i64,
    trash_count: i64,
    derived_bytes: i64,
    derived_count: i64,
}

/// The single aggregate query, parameterised only by its `WHERE` clause so the
/// per-account and whole-instance paths cannot drift apart in their definitions.
const TOTALS_SELECT: &str = "SELECT owner_id,
            COALESCE(SUM(size_bytes) FILTER (WHERE NOT is_trashed), 0)::bigint,
            (COUNT(*) FILTER (WHERE NOT is_trashed))::bigint,
            COALESCE(SUM(size_bytes) FILTER (WHERE is_trashed), 0)::bigint,
            (COUNT(*) FILTER (WHERE is_trashed))::bigint,
            COALESCE(SUM(derived_bytes), 0)::bigint,
            (COUNT(*) FILTER (WHERE has_thumbnail) + COUNT(*) FILTER (WHERE has_preview))::bigint
       FROM photos.photos";

type TotalsRow = (Uuid, i64, i64, i64, i64, i64, i64);

fn row_to_totals(r: &TotalsRow) -> Totals {
    Totals {
        content_bytes: r.1,
        content_count: r.2,
        trash_bytes: r.3,
        trash_count: r.4,
        derived_bytes: r.5,
        derived_count: r.6,
    }
}

/// Expands one account's aggregates into the lines actually declared.
///
/// All three categories are emitted even at zero. "Photos holds nothing for this
/// person" is a statement that has to be made explicitly, otherwise the last
/// non-zero figure would stand forever — that is precisely what happens when
/// somebody empties their bin and nothing declares the `trash` line back to zero.
fn entries_for(user_id: Uuid, t: &Totals) -> Vec<Entry> {
    vec![
        Entry {
            user_id,
            category: CAT_CONTENT,
            used_bytes: t.content_bytes,
            object_count: t.content_count,
        },
        Entry {
            user_id,
            category: CAT_TRASH,
            used_bytes: t.trash_bytes,
            object_count: t.trash_count,
        },
        Entry {
            user_id,
            category: CAT_THUMBNAILS,
            used_bytes: t.derived_bytes,
            object_count: t.derived_count,
        },
    ]
}

/// Recounts the named accounts.
///
/// Accounts with no photos at all are absent from the `GROUP BY`, so they are
/// filled back in at zero — see [`entries_for`].
async fn totals_for(db: &PgPool, owners: &[Uuid]) -> Result<Vec<Entry>, sqlx::Error> {
    let sql = format!("{TOTALS_SELECT} WHERE owner_id = ANY($1) GROUP BY owner_id");
    let rows: Vec<TotalsRow> = sqlx::query_as(&sql).bind(owners).fetch_all(db).await?;

    let found: HashMap<Uuid, Totals> = rows.iter().map(|r| (r.0, row_to_totals(r))).collect();
    let zero = Totals {
        content_bytes: 0,
        content_count: 0,
        trash_bytes: 0,
        trash_count: 0,
        derived_bytes: 0,
        derived_count: 0,
    };

    Ok(owners
        .iter()
        .flat_map(|o| entries_for(*o, found.get(o).unwrap_or(&zero)))
        .collect())
}

/// Recounts every account photos holds anything for.
async fn totals_all(db: &PgPool) -> Result<Vec<Entry>, sqlx::Error> {
    let sql = format!("{TOTALS_SELECT} GROUP BY owner_id");
    let rows: Vec<TotalsRow> = sqlx::query_as(&sql).fetch_all(db).await?;

    let mut entries: Vec<Entry> = rows
        .iter()
        .flat_map(|r| entries_for(r.0, &row_to_totals(r)))
        .collect();

    // Stable order so consecutive declarations chunk identically — a moving chunk
    // boundary would make partial declarations retire different accounts each time.
    entries.sort_by(|a, b| (a.user_id, a.category).cmp(&(b.user_id, b.category)));
    Ok(entries)
}

/// Measures the derivatives of rows written before `derived_bytes` existed.
///
/// Bounded by [`BACKFILL_BATCH`] × [`BACKFILL_MAX_BATCHES`] per call — see those
/// constants for why the ceiling is not optional.
///
/// A row whose derivatives turn out not to exist has its `has_thumbnail` /
/// `has_preview` cleared rather than being left at zero. Without that, `0` would
/// stay ambiguous between "not measured yet" and "genuinely nothing", and the
/// scan would re-measure the same broken rows every six hours forever.
async fn backfill_derived_bytes(db: &PgPool, storage: &dyn kubuno_storage::StorageBackend) {
    let mut repaired = 0usize;

    for _ in 0..BACKFILL_MAX_BATCHES {
        let rows: Vec<(Uuid, Uuid, bool, bool)> = match sqlx::query_as(
            "SELECT id, owner_id, has_thumbnail, has_preview
               FROM photos.photos
              WHERE derived_bytes = 0 AND (has_thumbnail OR has_preview)
              ORDER BY id
              LIMIT $1",
        )
        .bind(BACKFILL_BATCH)
        .fetch_all(db)
        .await
        {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(error = %e, "Lecture du lot de rattrapage des dérivés échouée");
                return;
            }
        };

        if rows.is_empty() {
            break;
        }
        let short_batch = (rows.len() as i64) < BACKFILL_BATCH;

        for (id, owner_id, has_thumbnail, has_preview) in rows {
            let mut total: i64 = 0;
            let mut thumb_ok = false;
            let mut preview_ok = false;

            if has_thumbnail {
                if let Ok(n) = storage
                    .size(&super::photo_service::thumbnail_path(owner_id, id))
                    .await
                {
                    total += n as i64;
                    thumb_ok = true;
                }
            }
            if has_preview {
                if let Ok(n) = storage
                    .size(&super::photo_service::preview_path(owner_id, id))
                    .await
                {
                    total += n as i64;
                    preview_ok = true;
                }
            }

            if let Err(e) = sqlx::query(
                "UPDATE photos.photos
                    SET derived_bytes = $1, has_thumbnail = $2, has_preview = $3
                  WHERE id = $4",
            )
            .bind(total)
            .bind(thumb_ok)
            .bind(preview_ok)
            .bind(id)
            .execute(db)
            .await
            {
                tracing::error!(error = %e, photo = %id, "Écriture du rattrapage des dérivés échouée");
                continue;
            }
            repaired += 1;
        }

        if short_batch {
            break;
        }
    }

    if repaired > 0 {
        tracing::info!(photos = repaired, "Rattrapage des tailles de dérivés effectué");
    }
}

/// How many calls a declaration of `n` entries takes. Zero entries still takes
/// one: an empty `full` declaration is a statement, not a no-op.
fn page_count(n: usize) -> usize {
    if n == 0 {
        1
    } else {
        n.div_ceil(MAX_ENTRIES)
    }
}

/// Whether `full` may actually be claimed on the wire.
///
/// A chunked declaration marked `full` would retire every entry outside whichever
/// chunk happened to be sent last. Beyond the ceiling it is therefore sent as
/// partial: correct, but unable to retire a line — a limitation the incremental
/// path already covers, since dropping to zero marks the owner dirty and zero is
/// declared explicitly.
fn claims_full(full: bool, n: usize) -> bool {
    full && n <= MAX_ENTRIES
}

/// Sends one declaration to the core.
async fn send(
    http: &reqwest::Client,
    state: &AppState,
    entries: &[Entry],
    full: bool,
) -> Result<(), String> {
    let url = format!("{}/internal/storage/usage", state.settings.core.url);
    let usage: Vec<_> = entries
        .iter()
        .map(|e| {
            json!({
                "user_id":      e.user_id,
                "category":     e.category,
                "used_bytes":   e.used_bytes,
                "object_count": e.object_count,
            })
        })
        .collect();

    let resp = http
        .post(&url)
        .header(
            "X-Internal-Secret",
            state.settings.core.internal_secret.as_str(),
        )
        .json(&json!({ "module_id": MODULE_ID, "full": full, "usage": usage }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        return Ok(());
    }

    // The status alone does not say which of several validations refused the
    // declaration, and this runs unattended — a log line reading "HTTP 422" costs
    // an afternoon the next time the contract shifts.
    let status = resp.status();
    let detail = resp.text().await.unwrap_or_default();
    let detail: String = detail.chars().take(300).collect();
    Err(format!("HTTP {status} {detail}"))
}

/// Declares `entries` in as many calls as the core's ceiling requires.
///
/// Returns `true` when every call landed; the caller uses that to decide whether
/// a full sync needs retrying sooner than its normal period.
async fn declare(http: &reqwest::Client, state: &AppState, entries: Vec<Entry>, full: bool) -> bool {
    let full_on_wire = claims_full(full, entries.len());
    if full && !full_on_wire {
        tracing::warn!(
            entrées = entries.len(),
            envois = page_count(entries.len()),
            "Synchronisation complète découpée : déclarée en plusieurs envois partiels"
        );
    }

    // An empty full declaration is meaningful and must still be sent: it is how
    // photos says "I hold nothing for anybody", which the core has to be able to
    // tell apart from "photos has never declared".
    if entries.is_empty() {
        if full {
            return match send(http, state, &[], true).await {
                Ok(()) => {
                    tracing::debug!("Consommation déclarée : aucun compte");
                    true
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Déclaration de consommation échouée");
                    false
                }
            };
        }
        return true;
    }

    let mut declared_bytes: i64 = 0;
    let mut sent = 0usize;
    let mut all_ok = true;
    for chunk in entries.chunks(MAX_ENTRIES) {
        match send(http, state, chunk, full_on_wire).await {
            Ok(()) => {
                declared_bytes += chunk.iter().map(|e| e.used_bytes).sum::<i64>();
                sent += chunk.len();
            }
            Err(e) => {
                // Incremental declarations are never retried here: the next flush
                // declares the current state again, and retrying a stale total
                // would be work done to publish an out-of-date number. A *full*
                // declaration is different — it is the repair path, and its caller
                // reschedules it on this return value.
                all_ok = false;
                tracing::warn!(error = %e, entrées = chunk.len(), "Déclaration de consommation échouée");
            }
        }
    }

    if sent > 0 {
        tracing::debug!(
            entrées = sent,
            octets = declared_bytes,
            complète = full_on_wire,
            "Consommation déclarée au core"
        );
    }
    all_ok
}

/// The reporter task. Started once at bootstrap.
pub async fn run_reporter(state: AppState) {
    let (tx, mut rx) = mpsc::unbounded_channel::<Uuid>();
    if DIRTY.set(tx).is_err() {
        tracing::error!("Rapporteur de consommation déjà démarré — second démarrage ignoré");
        return;
    }

    let http = state.http.clone();
    let mut pending: HashSet<Uuid> = HashSet::new();

    // Absolute deadlines rather than `interval`s, so a failed full sync can be
    // pulled forward without disturbing the incremental rhythm.
    let mut flush_at = tokio::time::Instant::now() + FLUSH_INTERVAL;
    let mut full_at = tokio::time::Instant::now(); // the first one is immediate
    let mut full_backoff = FULL_RETRY_MIN;

    tracing::info!("Rapporteur de consommation démarré (déclaration au core)");

    loop {
        tokio::select! {
            // The channel's sender lives in a `static` for the process's whole
            // life, so `recv` never yields `None` and this branch never retires.
            Some(owner) = rx.recv() => {
                pending.insert(owner);
            }

            _ = tokio::time::sleep_until(flush_at) => {
                flush_at = tokio::time::Instant::now() + FLUSH_INTERVAL;
                if pending.is_empty() {
                    continue;
                }
                let owners: Vec<Uuid> = pending.drain().collect();
                match totals_for(&state.db, &owners).await {
                    Ok(entries) => { declare(&http, &state, entries, false).await; }
                    Err(e) => tracing::error!(
                        error = %e,
                        comptes = owners.len(),
                        "Recomptage de la consommation échoué"
                    ),
                }
            }

            // Fires immediately at startup — the first thing the module does once
            // it is up is state its complete position. Retried with a growing
            // backoff until it lands, because the core is often not ready to
            // accept it on the first attempt.
            _ = tokio::time::sleep_until(full_at) => {
                // Repair missing derivative measurements first, so the figures the
                // full sync publishes are the repaired ones rather than one period
                // behind.
                backfill_derived_bytes(&state.db, state.storage.as_ref()).await;

                let delivered = match totals_all(&state.db).await {
                    Ok(entries) => {
                        // A full declaration supersedes everything queued.
                        pending.clear();
                        declare(&http, &state, entries, true).await
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Recomptage complet de la consommation échoué");
                        false
                    }
                };

                let now = tokio::time::Instant::now();
                if delivered {
                    full_at = now + FULL_SYNC_INTERVAL;
                    full_backoff = FULL_RETRY_MIN;
                } else {
                    full_at = now + full_backoff;
                    full_backoff = (full_backoff * 2).min(FULL_SYNC_INTERVAL);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn totals(content: (i64, i64), trash: (i64, i64), derived: (i64, i64)) -> Totals {
        Totals {
            content_bytes: content.0,
            content_count: content.1,
            trash_bytes: trash.0,
            trash_count: trash.1,
            derived_bytes: derived.0,
            derived_count: derived.1,
        }
    }

    /// Only categories from the core's closed vocabulary, and exactly the three
    /// that describe what photos physically holds.
    #[test]
    fn emitted_categories_are_the_closed_vocabulary() {
        let vocabulary = [
            "content",
            "trash",
            "versions",
            "retention",
            "thumbnails",
            "index",
            "cache",
            "staging",
            "system",
            "delegated",
        ];
        let e = entries_for(Uuid::nil(), &totals((1, 1), (2, 2), (3, 4)));
        for entry in &e {
            assert!(
                vocabulary.contains(&entry.category),
                "category outside the closed vocabulary: {}",
                entry.category
            );
        }
        assert_eq!(
            e.iter().map(|x| x.category).collect::<Vec<_>>(),
            vec![CAT_CONTENT, CAT_TRASH, CAT_THUMBNAILS]
        );
        // Photos holds its own bytes, so unlike office it never emits `delegated`.
        assert!(!e.iter().any(|x| x.category == "delegated"));
        // Photos keeps no history of any kind — neither a purgeable one nor one it
        // retains on its own policy.
        assert!(!e.iter().any(|x| x.category == "versions" || x.category == "retention"));
    }

    /// A trashed photo must stay billed and stay on its own line: it still occupies
    /// the disk and the person can still free it, and folding it into `content`
    /// would hide the one fact that explains a saturated account.
    #[test]
    fn trash_is_declared_separately_and_never_folded_into_content() {
        let e = entries_for(Uuid::nil(), &totals((100, 1), (250, 2), (0, 0)));
        let content = e.iter().find(|x| x.category == CAT_CONTENT).expect("content");
        let trash = e.iter().find(|x| x.category == CAT_TRASH).expect("trash");
        assert_eq!(content.used_bytes, 100);
        assert_eq!(trash.used_bytes, 250);
        // Both are billed, so the quota consequence is their sum — never one or
        // the other.
        assert_eq!(content.used_bytes + trash.used_bytes, 350);
    }

    /// Every account is declared on all three lines, at zero when empty — that is
    /// what retires a stale figure after somebody empties their bin.
    #[test]
    fn empty_account_still_declares_every_category_at_zero() {
        let e = entries_for(Uuid::nil(), &totals((0, 0), (0, 0), (0, 0)));
        assert_eq!(e.len(), 3);
        assert!(e.iter().all(|x| x.used_bytes == 0 && x.object_count == 0));
    }

    /// Derivatives are counted per *file*, not per photo: a photo with both a
    /// thumbnail and a preview is two objects on disk.
    #[test]
    fn derivative_objects_are_counted_per_file() {
        let e = entries_for(Uuid::nil(), &totals((0, 0), (0, 0), (9_000, 4)));
        let thumbs = e
            .iter()
            .find(|x| x.category == CAT_THUMBNAILS)
            .expect("thumbnails");
        assert_eq!(thumbs.used_bytes, 9_000);
        assert_eq!(thumbs.object_count, 4);
    }

    #[test]
    fn paging_respects_the_core_ceiling() {
        // Zero entries is one call, not none: an empty full declaration says
        // "photos holds nothing", which the core must receive.
        assert_eq!(page_count(0), 1);
        assert_eq!(page_count(1), 1);
        assert_eq!(page_count(MAX_ENTRIES), 1);
        assert_eq!(page_count(MAX_ENTRIES + 1), 2);
        assert_eq!(page_count(3 * MAX_ENTRIES), 3);
        assert_eq!(page_count(3 * MAX_ENTRIES + 1), 4);
    }

    #[test]
    fn full_is_only_claimed_on_a_single_call() {
        assert!(claims_full(true, 0));
        assert!(claims_full(true, MAX_ENTRIES));
        assert!(!claims_full(true, MAX_ENTRIES + 1));
        assert!(!claims_full(false, 1));
    }

    /// Three lines per account means the ceiling is reached three times sooner
    /// than a per-account count would suggest — worth pinning so nobody sizes the
    /// chunking against accounts instead of entries.
    #[test]
    fn chunking_covers_every_entry_exactly_once() {
        let entries: Vec<Entry> = (0..MAX_ENTRIES + 7)
            .map(|i| Entry {
                user_id: Uuid::from_u128(i as u128 + 1),
                category: CAT_CONTENT,
                used_bytes: 1,
                object_count: 1,
            })
            .collect();

        let chunks: Vec<&[Entry]> = entries.chunks(MAX_ENTRIES).collect();
        assert_eq!(chunks.len(), page_count(entries.len()));
        assert_eq!(chunks.iter().map(|c| c.len()).sum::<usize>(), entries.len());
        assert_eq!(chunks[0].len(), MAX_ENTRIES);
        assert_eq!(chunks[1].len(), 7);
    }

    /// The backfill has to stay bounded: it costs one `size()` round-trip per
    /// derivative against a backend that may be an object store.
    #[test]
    fn backfill_is_bounded() {
        assert!(BACKFILL_BATCH > 0);
        assert!(BACKFILL_MAX_BATCHES > 0);
        assert!(BACKFILL_BATCH * BACKFILL_MAX_BATCHES as i64 <= 10_000);
    }

    /// Photos declares only its own tables — nothing it reads may belong to
    /// another module, or the same bytes would be counted twice.
    #[test]
    fn totals_query_reads_only_the_photos_schema() {
        assert!(TOTALS_SELECT.contains("photos.photos"));
        assert!(!TOTALS_SELECT.contains("drive."));
        assert!(!TOTALS_SELECT.contains("core."));
    }
}
