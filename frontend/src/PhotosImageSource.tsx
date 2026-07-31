import { useEffect, useMemo, useState } from 'react'
import { photosApi, type Photo } from './api'
import type { ImageSourceProps } from '@kubuno/sdk'

/**
 * The "Photos" tab of the core image picker. Registered by this module, so the
 * tab only exists when photos is installed — the core never imports us.
 *
 * The search box lives in the DIALOG header; we only receive its text.
 */

const DATE_FMT = new Intl.DateTimeFormat('fr-FR', { day: 'numeric', month: 'long', year: 'numeric' })

/** Groups photos by capture day, newest first — the library's own reading order. */
function groupByDay(photos: Photo[]): Array<{ day: string; label: string; photos: Photo[] }> {
  const buckets = new Map<string, { label: string; photos: Photo[] }>()
  for (const p of photos) {
    const when = new Date(p.taken_at ?? p.created_at)
    const day  = Number.isNaN(when.getTime()) ? '' : when.toISOString().slice(0, 10)
    const label = day ? DATE_FMT.format(when) : 'Sans date'
    const bucket = buckets.get(day)
    if (bucket) bucket.photos.push(p)
    else buckets.set(day, { label, photos: [p] })
  }
  return [...buckets.entries()]
    .sort((a, b) => b[0].localeCompare(a[0]))
    .map(([day, v]) => ({ day, ...v }))
}

export default function PhotosImageSource({ onPick, query }: ImageSourceProps) {
  const [photos, setPhotos]   = useState<Photo[]>([])
  const [loading, setLoading] = useState(true)
  const [error, setError]     = useState<string | null>(null)

  useEffect(() => {
    let cancelled = false
    setLoading(true)
    // Debounced so typing in the dialog's search box does not hammer the API.
    const id = window.setTimeout(() => {
      photosApi.listPhotos({ search: query.trim() || undefined, limit: 120 })
        .then(r => { if (!cancelled) { setPhotos(r.photos); setError(null) } })
        .catch(() => { if (!cancelled) setError('Impossible de charger vos photos.') })
        .finally(() => { if (!cancelled) setLoading(false) })
    }, query ? 250 : 0)
    return () => { cancelled = true; window.clearTimeout(id) }
  }, [query])

  const groups = useMemo(() => groupByDay(photos), [photos])

  if (loading) return <p className="text-sm text-text-tertiary">Chargement…</p>
  if (error)   return <p className="text-sm text-danger">{error}</p>
  if (!photos.length) {
    return <p className="text-sm text-text-tertiary">{query ? 'Aucun résultat.' : 'Aucune photo.'}</p>
  }

  return (
    <div className="space-y-5">
      {groups.map(g => (
        <section key={g.day}>
          <h3 className="text-sm text-text-secondary mb-2">{g.label}</h3>
          <div className="flex flex-wrap gap-2">
            {g.photos.map(p => {
              // Keep the photo's own proportions on a fixed row height, the way
              // a photo library lays out — never square-cropped.
              const ratio = p.width && p.height ? p.width / p.height : 1
              return (
                <button
                  key={p.id}
                  type="button"
                  title={p.original_name}
                  onClick={() => onPick({ kind: 'url', url: photosApi.downloadUrl(p.id) })}
                  style={{ width: `${Math.max(64, Math.min(220, 120 * ratio))}px` }}
                  className="h-[120px] shrink-0 rounded-lg overflow-hidden bg-surface-2 hover:opacity-80 transition-opacity"
                >
                  <img src={photosApi.thumbnailUrl(p.id)} alt={p.original_name}
                    loading="lazy" className="w-full h-full object-cover" />
                </button>
              )
            })}
          </div>
        </section>
      ))}
    </div>
  )
}
