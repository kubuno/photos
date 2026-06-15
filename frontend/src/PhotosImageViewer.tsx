import { useState, useEffect, useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { Download, X, ChevronLeft, ChevronRight, Pencil } from 'lucide-react'
import { useQueryClient } from '@tanstack/react-query'
import PhotoEditor, { type TransformState } from './PhotoEditor'
import { api } from '@kubuno/sdk'
import { formatSize } from '@kubuno/sdk'
import { bumpImageCache, useImageCacheStore } from '@kubuno/sdk'

export interface ViewerFile {
  id:         string
  name:       string
  mime_type:  string
  size_bytes: number
}

interface Props {
  file:       ViewerFile
  imageFiles: ViewerFile[]
  onClose:    () => void
}

function thumbnailUrl(id: string) { return `/api/v1/drive/${id}/thumbnail` }
function downloadUrl(id: string)  { return `/api/v1/drive/${id}/download` }

export default function PhotosImageViewer({ file, imageFiles, onClose }: Props) {
  const { t }          = useTranslation('photos')
  const initialIdx     = imageFiles.findIndex(f => f.id === file.id)
  const [idx, setIdx]  = useState(initialIdx < 0 ? 0 : initialIdx)
  const [editing, setEditing] = useState(false)
  const qc = useQueryClient()

  const current = imageFiles[idx] ?? file

  const [blobUrl, setBlobUrl] = useState<string | null>(null)
  const [blobKey, setBlobKey] = useState(0)
  const thumbVer = useImageCacheStore(s => s.versions[current.id] ?? 0)
  const thumbSrc = thumbVer
    ? `${thumbnailUrl(current.id)}?v=${thumbVer}`
    : thumbnailUrl(current.id)

  // Blob URL for high-quality view (authenticated). blobKey increments after each save.
  useEffect(() => {
    let active = true
    let url: string | null = null
    setBlobUrl(null)
    api.get(`/drive/${current.id}/download`, { responseType: 'blob' }).then(r => {
      if (!active) return
      url = URL.createObjectURL(r.data)
      setBlobUrl(url)
    })
    return () => { active = false; if (url) URL.revokeObjectURL(url) }
  }, [current.id, blobKey])

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (editing) return
      if (e.key === 'Escape')     onClose()
      if (e.key === 'ArrowLeft')  setIdx(i => Math.max(0, i - 1))
      if (e.key === 'ArrowRight') setIdx(i => Math.min(imageFiles.length - 1, i + 1))
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [onClose, imageFiles.length, editing])

  const handleSave = useCallback(async (transforms: TransformState) => {
    await api.post(`/drive/${current.id}/transform`, {
      rotate: transforms.rotate || null,
      flip_h: transforms.flipH || null,
      flip_v: transforms.flipV || null,
    })
    qc.invalidateQueries({ queryKey: ['files'] })
    qc.invalidateQueries({ queryKey: ['recent-files'] })
    bumpImageCache(current.id)   // bust thumbnail cache in all grids
    setEditing(false)
    setBlobKey(k => k + 1)       // re-fetch the full-quality blob
  }, [current.id, qc])

  return (
    <>
      <div
        className="fixed inset-0 z-50 bg-black/92 flex flex-col"
        onClick={onClose}
      >
        {/* Header */}
        <div
          className="flex items-center justify-between px-4 py-3 bg-black/60 shrink-0"
          onClick={e => e.stopPropagation()}
        >
          <div>
            <p className="text-white text-sm font-medium truncate max-w-[60vw]">{current.name}</p>
            <p className="text-white/50 text-xs">{formatSize(current.size_bytes)}</p>
          </div>
          <div className="flex items-center gap-2">
            {imageFiles.length > 1 && (
              <span className="text-white/50 text-xs">{idx + 1} / {imageFiles.length}</span>
            )}
            <button
              onClick={() => setEditing(true)}
              className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-white/10 hover:bg-white/20 text-white rounded-lg transition-colors"
              title={t('photos_viewer_edit_title')}
            >
              <Pencil size={14} />
              {t('common_edit')}
            </button>
            <a
              href={downloadUrl(current.id)}
              download={current.name}
              className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-white/10 hover:bg-white/20 text-white rounded-lg transition-colors"
              onClick={e => e.stopPropagation()}
            >
              <Download size={14} />
              {t('photos_viewer_download')}
            </a>
            <button
              onClick={onClose}
              className="p-2 hover:bg-white/10 rounded-full text-white transition-colors"
            >
              <X size={18} />
            </button>
          </div>
        </div>

        {/* Image */}
        <div
          className="flex-1 flex items-center justify-center relative overflow-hidden"
          onClick={e => e.stopPropagation()}
        >
          {idx > 0 && (
            <button
              onClick={() => setIdx(i => i - 1)}
              className="absolute left-3 z-10 p-2 bg-black/40 hover:bg-black/60 rounded-full text-white transition-colors"
            >
              <ChevronLeft size={22} />
            </button>
          )}

          <img
            key={`${current.id}-${blobKey}`}
            src={blobUrl ?? thumbSrc}
            alt={current.name}
            className="max-h-full max-w-full object-contain"
            draggable={false}
          />

          {idx < imageFiles.length - 1 && (
            <button
              onClick={() => setIdx(i => i + 1)}
              className="absolute right-3 z-10 p-2 bg-black/40 hover:bg-black/60 rounded-full text-white transition-colors"
            >
              <ChevronRight size={22} />
            </button>
          )}
        </div>
      </div>

      {/* Editor overlay */}
      {editing && blobUrl && (
        <PhotoEditor
          imageUrl={blobUrl}
          fileName={current.name}
          onSave={handleSave}
          onCancel={() => setEditing(false)}
        />
      )}
    </>
  )
}
