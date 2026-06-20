import { useEffect, useRef, useState, useCallback } from 'react'
import { isCoarsePointer } from './openable'
import { useTranslation } from 'react-i18next'
import { useLocation } from 'react-router-dom'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { useConfirm } from '@kubuno/sdk'
import { ConfirmDialog } from '@ui'
import {
  Image, Heart, Trash2, Download,
  BookImage, X, Plus, ChevronLeft, ArrowLeft,
  Loader2, Upload, Pencil, Play,
} from 'lucide-react'
import { photosApi, type Photo, type Album } from './api'
import { useAuthStore } from '@kubuno/sdk'
import { Button, Input } from '@ui'

// Cache de blob URLs pour les previews vidéo — évite de re-fetcher à chaque hover
const videoPreviewCache = new Map<string, string>()
import { usePhotosStore } from './store'
import { format } from 'date-fns'
import { getDateLocale } from '@kubuno/sdk'
import PhotoEditor, { type TransformState } from './PhotoEditor'
import { api } from '@kubuno/sdk'
import { bumpImageCache, useImageCacheStore } from '@kubuno/sdk'

// ── Types de vue ──────────────────────────────────────────────────────────────
type ViewMode = 'timeline' | 'albums' | 'starred' | 'trash'

interface PhotosAppProps {
  starred?: boolean
  trashed?: boolean
  albumsView?: boolean
}

// ── PhotoCard ─────────────────────────────────────────────────────────────────
function PhotoCard({
  photo, selected, selectionMode, onSelect, onOpen, onStar, onTrash,
}: {
  photo: Photo
  selected: boolean
  selectionMode: boolean
  onSelect: (id: string, additive: boolean) => void
  onOpen: (photo: Photo) => void
  onStar: (id: string, starred: boolean) => void
  onTrash: (id: string) => void
}) {
  const thumbVer = useImageCacheStore(s => s.versions[photo.id] ?? 0)
  const thumbSrc = thumbVer
    ? `${photosApi.thumbnailUrl(photo.id)}?v=${thumbVer}`
    : photosApi.thumbnailUrl(photo.id)

  const isVideo    = photo.mime_type.startsWith('video/')
  const videoRef   = useRef<HTMLVideoElement>(null)
  const fetchingRef = useRef(false)
  const [videoPlaying, setVideoPlaying] = useState(false)

  const stopVideoPreview = useCallback(() => {
    const vid = videoRef.current
    if (vid) { vid.pause(); vid.currentTime = 0 }
    setVideoPlaying(false)
  }, [])

  // Le <video> ne peut pas envoyer le header Authorization directement.
  // On fetche les premiers 10 Mo avec fetch() + Bearer token, on crée un blob URL
  // mis en cache par photo.id pour ne pas re-télécharger à chaque survol.
  const startVideoPreview = useCallback(async () => {
    if (!isVideo || fetchingRef.current) return
    const vid = videoRef.current
    if (!vid) return

    let blobUrl = videoPreviewCache.get(photo.id)

    if (!blobUrl) {
      fetchingRef.current = true
      const token = useAuthStore.getState().accessToken
      if (!token) { fetchingRef.current = false; return }
      try {
        const resp = await fetch(photosApi.downloadUrl(photo.id), {
          headers: {
            'Authorization': `Bearer ${token}`,
            'Range': 'bytes=0-10485760', // 10 Mo — couvre le moov atom des MP4 faststart
          },
        })
        if (!resp.ok && resp.status !== 206) throw new Error('fetch failed')
        const blob = await resp.blob()
        blobUrl = URL.createObjectURL(blob)
        videoPreviewCache.set(photo.id, blobUrl)
      } catch {
        fetchingRef.current = false
        return
      }
      fetchingRef.current = false
    }

    if (!videoRef.current) return
    if (videoRef.current.src !== blobUrl) {
      videoRef.current.src = blobUrl
    }
    videoRef.current.currentTime = 0
    videoRef.current.play().then(() => setVideoPlaying(true)).catch(() => {})
  }, [isVideo, photo.id])

  useEffect(() => {
    if (!isVideo) return
    if (selected) startVideoPreview()
    else stopVideoPreview()
  }, [selected, isVideo, startVideoPreview, stopVideoPreview])

  const handleVideoMouseEnter = () => { if (!selected) startVideoPreview() }
  const handleVideoMouseLeave = () => { if (!selected) stopVideoPreview() }
  const handleVideoTimeUpdate = () => {
    if (videoRef.current && videoRef.current.currentTime >= 5) stopVideoPreview()
  }
  const handleVideoEnded = () => stopVideoPreview()

  const handleClick = (e: React.MouseEvent) => {
    if (selectionMode || e.ctrlKey || e.metaKey) {
      onSelect(photo.id, true)
      return
    }
    // Touch UIs have no double-click: a single tap opens the photo.
    if (isCoarsePointer()) { e.preventDefault(); onOpen(photo) }
  }
  const handleDoubleClick = (e: React.MouseEvent) => {
    if (selectionMode || isCoarsePointer()) return
    e.preventDefault()
    onOpen(photo)
  }

  return (
    <div
      // select-none prevents browser text-selection on double-click
      className={`relative group aspect-square rounded-lg overflow-hidden cursor-pointer border-2 transition-all select-none
        ${selected ? 'border-primary ring-2 ring-primary/30' : 'border-transparent hover:shadow-md'}
      `}
      onClick={handleClick}
      onDoubleClick={handleDoubleClick}
      onMouseEnter={isVideo ? handleVideoMouseEnter : undefined}
      onMouseLeave={isVideo ? handleVideoMouseLeave : undefined}
    >
      <img
        src={thumbSrc}
        alt={photo.original_name}
        className={`w-full h-full object-cover pointer-events-none transition-opacity duration-200 ${videoPlaying ? 'opacity-0' : 'opacity-100'}`}
        loading="lazy"
        draggable={false}
      />
      {isVideo && (
        <video
          ref={videoRef}
          muted
          playsInline
          preload="none"
          onTimeUpdate={handleVideoTimeUpdate}
          onEnded={handleVideoEnded}
          className={`absolute inset-0 w-full h-full object-cover transition-opacity duration-200 ${videoPlaying ? 'opacity-100' : 'opacity-0'}`}
        />
      )}
      {isVideo && !videoPlaying && (
        <div className="absolute bottom-1.5 right-1.5 pointer-events-none">
          <div className="w-5 h-5 rounded-full bg-black/50 flex items-center justify-center">
            <Play size={10} className="text-white ml-0.5" fill="white" />
          </div>
        </div>
      )}

      {/* Checkbox — always visible when selected, visible on hover otherwise */}
      <div className={`absolute top-2 left-2 transition-opacity ${selected ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'}`}>
        <button
          onClick={e => { e.stopPropagation(); onSelect(photo.id, true) }}
          className={`w-5 h-5 rounded-full border-2 border-white flex items-center justify-center transition-colors shadow-sm
            ${selected ? 'bg-primary border-primary' : 'bg-black/30'}`}
        >
          {selected && <span className="text-white text-[10px] font-bold leading-none">✓</span>}
        </button>
      </div>

      {/* Action buttons — visible on hover, hidden when already selected */}
      {!selected && (
        <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition-opacity flex gap-1 no-print">
          <button
            onClick={e => { e.stopPropagation(); onStar(photo.id, !photo.is_starred) }}
            className="w-7 h-7 rounded-full bg-white/80 flex items-center justify-center hover:bg-white shadow-sm"
          >
            <Heart size={14} className={photo.is_starred ? 'fill-red-500 text-red-500' : 'text-text-secondary'} />
          </button>
          <button
            onClick={e => { e.stopPropagation(); onTrash(photo.id) }}
            className="w-7 h-7 rounded-full bg-white/80 flex items-center justify-center hover:bg-white shadow-sm"
          >
            <Trash2 size={14} className="text-text-secondary" />
          </button>
        </div>
      )}

      {photo.is_starred && !selected && (
        <div className="absolute bottom-1 right-1 group-hover:opacity-0 transition-opacity">
          <Heart size={14} className="fill-red-500 text-red-500 drop-shadow" />
        </div>
      )}
    </div>
  )
}

// ── PhotosLightbox — viewer avec éditeur intégré ──────────────────────────────
function PhotosLightbox({
  photo, photos, onClose, onNavigate,
}: {
  photo: Photo
  photos: Photo[]
  onClose: () => void
  onNavigate: (photo: Photo) => void
}) {
  const { t, i18n } = useTranslation('photos')
  const idx    = photos.findIndex(p => p.id === photo.id)
  const qc     = useQueryClient()
  const [editing,  setEditing]  = useState(false)
  const [blobUrl,  setBlobUrl]  = useState<string | null>(null)
  const [loading,  setLoading]  = useState(true)
  const [blobKey,  setBlobKey]  = useState(0)

  // Use preview (always JPEG) so the browser can display it regardless of original format.
  // blobKey increments after each save to force a fresh fetch.
  useEffect(() => {
    let active = true
    let url: string | null = null
    setLoading(true)
    setBlobUrl(null)
    api.get(`/photos/${photo.id}/preview`, { responseType: 'blob' })
      .then(r => {
        if (!active) return
        url = URL.createObjectURL(r.data)
        setBlobUrl(url)
      })
      .finally(() => { if (active) setLoading(false) })
    return () => { active = false; if (url) URL.revokeObjectURL(url) }
  }, [photo.id, blobKey])

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (editing) return
      if (e.key === 'Escape')     onClose()
      if (e.key === 'ArrowLeft'  && idx > 0)                 onNavigate(photos[idx - 1])
      if (e.key === 'ArrowRight' && idx < photos.length - 1) onNavigate(photos[idx + 1])
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [onClose, onNavigate, idx, photos, editing])

  const handleSave = async (transforms: TransformState) => {
    await api.post(`/photos/${photo.id}/transform`, {
      rotate: transforms.rotate || null,
      flip_h: transforms.flipH  || null,
      flip_v: transforms.flipV  || null,
    })
    qc.invalidateQueries({ queryKey: ['photos'] })
    bumpImageCache(photo.id)   // bust thumbnail cache in all grids
    setEditing(false)
    setBlobKey(k => k + 1)     // re-fetch the preview blob
  }

  return (
    <>
      <div className="fixed inset-0 z-50 bg-black/95 flex flex-col select-none" onClick={onClose}>
        {/* Barre du haut */}
        <div
          className="flex items-center justify-between px-4 py-3 text-white shrink-0 no-print"
          onClick={e => e.stopPropagation()}
        >
          <div className="flex items-center gap-3 min-w-0 flex-1">
            <button onClick={onClose} className="hover:bg-white/10 p-2 rounded-full flex-shrink-0">
              <X size={20} />
            </button>
            <span className="text-sm opacity-60 flex-shrink-0">{idx + 1} / {photos.length}</span>
            <span className="text-sm font-medium truncate">{photo.original_name}</span>
          </div>
          <div className="flex items-center gap-2 flex-shrink-0">
            {photo.taken_at && (
              <span className="text-sm opacity-50 hidden sm:block">
                {format(new Date(photo.taken_at), 'dd MMM yyyy', { locale: getDateLocale(i18n.language) })}
              </span>
            )}
            <button
              onClick={() => setEditing(true)}
              className="flex items-center gap-1.5 px-3 py-1.5 text-sm bg-white/10 hover:bg-white/20 rounded-lg transition-colors"
            >
              <Pencil size={14} />
              <span className="hidden sm:inline">{t('photos_edit')}</span>
            </button>
            <a
              href={photosApi.downloadUrl(photo.id)}
              download={photo.original_name}
              className="hover:bg-white/10 p-2 rounded-full"
              onClick={e => e.stopPropagation()}
            >
              <Download size={18} />
            </a>
          </div>
        </div>

        {/* Image */}
        <div
          className="flex-1 flex items-center justify-center relative overflow-hidden"
          onClick={e => e.stopPropagation()}
        >
          {idx > 0 && (
            <button
              onClick={() => onNavigate(photos[idx - 1])}
              className="absolute left-4 p-3 rounded-full bg-white/10 hover:bg-white/20 text-white z-10"
            >
              <ChevronLeft size={24} />
            </button>
          )}

          {loading && <Loader2 size={28} className="text-white/50 animate-spin" />}
          {blobUrl && (
            <img
              key={photo.id}
              src={blobUrl}
              alt={photo.original_name}
              className="max-w-full max-h-full object-contain"
              style={{ maxHeight: 'calc(100vh - 120px)' }}
              draggable={false}
            />
          )}

          {idx < photos.length - 1 && (
            <button
              onClick={() => onNavigate(photos[idx + 1])}
              className="absolute right-4 p-3 rounded-full bg-white/10 hover:bg-white/20 text-white z-10"
            >
              <ChevronLeft size={24} className="rotate-180" />
            </button>
          )}
        </div>

        {/* Métadonnées */}
        {(photo.camera_make || photo.camera_model || photo.width) && (
          <div
            className="flex items-center gap-4 px-4 py-3 text-white/50 text-xs shrink-0"
            onClick={e => e.stopPropagation()}
          >
            {(photo.camera_make || photo.camera_model) && (
              <span>{[photo.camera_make, photo.camera_model].filter(Boolean).join(' ')}</span>
            )}
            {photo.width && photo.height && <span>{photo.width} × {photo.height}</span>}
            <span>{t('photos_megabytes', { value: (photo.size_bytes / 1_000_000).toFixed(1) })}</span>
          </div>
        )}
      </div>

      {/* Éditeur (par-dessus la lightbox) */}
      {editing && blobUrl && (
        <PhotoEditor
          imageUrl={blobUrl}
          fileName={photo.original_name}
          onSave={handleSave}
          onCancel={() => setEditing(false)}
        />
      )}
    </>
  )
}

// ── AlbumCard ─────────────────────────────────────────────────────────────────
function AlbumCard({ album, onClick }: { album: Album; onClick: () => void }) {
  const { t } = useTranslation('photos')
  return (
    <div className="cursor-pointer group" onClick={onClick}>
      <div className="aspect-square rounded-xl bg-surface-2 overflow-hidden border border-border mb-2 group-hover:shadow-md transition-shadow flex items-center justify-center">
        {album.cover_photo_id
          ? <img src={photosApi.thumbnailUrl(album.cover_photo_id)} alt={album.name} className="w-full h-full object-cover" />
          : <BookImage size={40} className="text-text-tertiary" />
        }
      </div>
      <p className="text-sm font-medium text-text-primary truncate">{album.name}</p>
      <p className="text-xs text-text-secondary">{t('photos_photo_count', { count: album.photo_count })}</p>
    </div>
  )
}

// ── Création d'album ──────────────────────────────────────────────────────────
function CreateAlbumModal({ onClose, onCreate }: { onClose: () => void; onCreate: (name: string) => void }) {
  const { t } = useTranslation('photos')
  const [name, setName] = useState('')
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4">
      <div className="bg-white rounded-2xl shadow-xl w-full max-w-sm p-6" onClick={e => e.stopPropagation()}>
        <h2 className="text-lg font-semibold text-text-primary mb-4">{t('new_album')}</h2>
        <div className="mb-4">
          <Input
            autoFocus type="text" placeholder={t('album_name')} value={name}
            onChange={e => setName(e.target.value)}
            onKeyDown={e => { if (e.key === 'Enter' && name.trim()) { onCreate(name.trim()); onClose() } }}
          />
        </div>
        <div className="flex gap-2 justify-end">
          <Button variant="ghost" onClick={onClose}>{t('cancel')}</Button>
          <Button disabled={!name.trim()} onClick={() => { if (name.trim()) { onCreate(name.trim()); onClose() } }}>
            {t('photos_create')}
          </Button>
        </div>
      </div>
    </div>
  )
}

// ── Composant principal ───────────────────────────────────────────────────────
export default function PhotosApp({ starred, trashed, albumsView }: PhotosAppProps) {
  const { t, i18n } = useTranslation('photos')
  const location    = useLocation()
  const queryClient = useQueryClient()

  const isAlbums  = albumsView || location.pathname === '/photos/albums'
  const isStarred = starred    || location.pathname === '/photos/starred'
  const isTrashed = trashed    || location.pathname === '/photos/trash'

  const [selectedPhotos,   setSelectedPhotos]   = useState<Set<string>>(new Set())
  const [lightboxPhoto,    setLightboxPhoto]     = useState<Photo | null>(null)
  const [activeAlbum,      setActiveAlbum]       = useState<Album | null>(null)
  const [showCreateAlbum,  setShowCreateAlbum]   = useState(false)
  const [uploading,        setUploading]         = useState(false)
  // Drag-and-drop
  const [isDragging,       setIsDragging]        = useState(false)
  const dragCounter = useRef(0)
  const { confirm, confirmState, handleConfirm, handleCancel } = useConfirm()

  const fileInputRef        = useRef<HTMLInputElement>(null)
  const registerFileInput   = usePhotosStore(s => s.registerFileInput)
  const createAlbumTrigger  = usePhotosStore(s => s.createAlbumTrigger)

  useEffect(() => { registerFileInput(() => fileInputRef.current?.click()) }, [registerFileInput])
  useEffect(() => { if (createAlbumTrigger > 0 && isAlbums) setShowCreateAlbum(true) }, [createAlbumTrigger, isAlbums])

  // ── Queries ──────────────────────────────────────────────────────────────────
  const photosQuery = useQuery({
    queryKey: ['photos', { starred: isStarred, trashed: isTrashed, albumId: activeAlbum?.id }],
    queryFn: () => photosApi.listPhotos({
      starred: isStarred || undefined,
      trashed: isTrashed || undefined,
      album_id: activeAlbum?.id,
      limit: 200,
    }),
    enabled: !isAlbums || !!activeAlbum,
  })

  const albumsQuery = useQuery({
    queryKey: ['photos-albums'],
    queryFn: photosApi.listAlbums,
    enabled: isAlbums,
  })

  const photos = photosQuery.data?.photos ?? []
  const albums = albumsQuery.data?.albums ?? []

  // ── Mutations ─────────────────────────────────────────────────────────────────
  const starMutation = useMutation({
    mutationFn: ({ id, starred }: { id: string; starred: boolean }) => photosApi.updatePhoto(id, { is_starred: starred }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['photos'] }),
  })
  const trashMutation = useMutation({
    mutationFn: (id: string) => photosApi.trashPhoto(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['photos'] }),
  })
  const restoreMutation = useMutation({
    mutationFn: (id: string) => photosApi.restorePhoto(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['photos'] }),
  })
  const deleteMutation = useMutation({
    mutationFn: (id: string) => photosApi.deletePhoto(id),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['photos'] }),
  })
  const createAlbumMutation = useMutation({
    mutationFn: (name: string) => photosApi.createAlbum({ name }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ['photos-albums'] }),
  })

  // ── Handlers ─────────────────────────────────────────────────────────────────
  const handleSelect = (id: string, additive: boolean) => {
    setSelectedPhotos(prev => {
      const next = new Set(prev)
      if (additive) {
        if (next.has(id)) next.delete(id); else next.add(id)
      } else {
        if (next.has(id) && next.size === 1) next.clear()
        else { next.clear(); next.add(id) }
      }
      return next
    })
  }

  const uploadFiles = async (files: File[]) => {
    const images = files.filter(f => f.type.startsWith('image/'))
    if (!images.length) return
    setUploading(true)
    try {
      for (const file of images) await photosApi.uploadPhoto(file)
      queryClient.invalidateQueries({ queryKey: ['photos'] })
    } finally {
      setUploading(false)
    }
  }

  const handleFileInput = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(e.target.files ?? [])
    await uploadFiles(files)
    if (fileInputRef.current) fileInputRef.current.value = ''
  }

  // ── Drag-and-drop ─────────────────────────────────────────────────────────────
  const handleDragEnter = (e: React.DragEvent) => {
    e.preventDefault()
    dragCounter.current++
    if (dragCounter.current === 1) setIsDragging(true)
  }
  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault()
    dragCounter.current--
    if (dragCounter.current === 0) setIsDragging(false)
  }
  const handleDragOver = (e: React.DragEvent) => { e.preventDefault() }
  const handleDrop = async (e: React.DragEvent) => {
    e.preventDefault()
    dragCounter.current = 0
    setIsDragging(false)
    await uploadFiles(Array.from(e.dataTransfer.files))
  }

  const openLightbox = (photo: Photo) => {
    setLightboxPhoto(photo)
    setSelectedPhotos(new Set())
  }

  const currentView: ViewMode = isAlbums ? 'albums' : isStarred ? 'starred' : isTrashed ? 'trash' : 'timeline'

  const photosByMonth = photos.reduce<Record<string, Photo[]>>((acc, photo) => {
    const key = format(new Date(photo.taken_at ?? photo.created_at), 'MMMM yyyy', { locale: getDateLocale(i18n.language) })
    if (!acc[key]) acc[key] = []
    acc[key].push(photo)
    return acc
  }, {})

  const titleMap: Record<ViewMode, string> = {
    timeline: t('photos_title_photos'),
    albums:   t('photos_title_albums'),
    starred:  t('photos_title_starred'),
    trash:    t('photos_title_trash'),
  }

  return (
    <div
      className="flex flex-col h-full relative"
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
    >
      {/* Overlay drag-and-drop */}
      {isDragging && !isTrashed && !isAlbums && (
        <div className="absolute inset-0 z-20 bg-primary/10 border-4 border-dashed border-primary rounded-xl flex flex-col items-center justify-center pointer-events-none">
          <Upload size={52} className="text-primary mb-3" />
          <p className="text-xl font-semibold text-primary">{t('drop_here')}</p>
          <p className="text-sm text-primary/70 mt-1">{t('formats')}</p>
        </div>
      )}

      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-border bg-white shrink-0">
        <div className="flex items-center gap-3">
          {activeAlbum && (
            <button onClick={() => setActiveAlbum(null)} className="p-1.5 rounded-lg hover:bg-surface-2 text-text-secondary mr-1">
              <ArrowLeft size={18} />
            </button>
          )}
          <h1 className="text-xl font-semibold text-text-primary">
            {activeAlbum ? activeAlbum.name : titleMap[currentView]}
          </h1>
          {photosQuery.data && !isAlbums && (
            <span className="text-sm text-text-tertiary">{t('photos_photo_count', { count: photos.length })}</span>
          )}
        </div>

        <div className="flex items-center gap-2">
          {selectedPhotos.size > 0 && (
            <div className="flex items-center gap-2 mr-2">
              <span className="text-sm text-text-secondary">{t('photos_selected_count', { count: selectedPhotos.size })}</span>
              {isTrashed ? (
                <>
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => { selectedPhotos.forEach(id => restoreMutation.mutate(id)); setSelectedPhotos(new Set()) }}
                  >{t('restore')}</Button>
                  <Button
                    variant="danger"
                    size="sm"
                    onClick={async () => {
                      const ok = await confirm({
                        title:        t('photos_delete_forever_title'),
                        message:      t('photos_delete_forever_message', { count: selectedPhotos.size }),
                        confirmLabel: t('photos_delete_forever_confirm'),
                        variant:      'danger',
                      })
                      if (ok) { selectedPhotos.forEach(id => deleteMutation.mutate(id)); setSelectedPhotos(new Set()) }
                    }}
                  >{t('delete')}</Button>
                </>
              ) : (
                <Button
                  variant="secondary"
                  size="sm"
                  onClick={() => { selectedPhotos.forEach(id => trashMutation.mutate(id)); setSelectedPhotos(new Set()) }}
                >{t('delete')}</Button>
              )}
            </div>
          )}

          {!isTrashed && !isAlbums && (
            <label
              htmlFor="photos-upload-input"
              className={`inline-flex items-center gap-2 h-8 px-3 text-sm font-medium rounded-md bg-primary text-white hover:bg-primary-hover transition-colors cursor-pointer select-none ${uploading ? 'opacity-50 pointer-events-none' : ''}`}
            >
              {uploading ? <Loader2 size={15} className="animate-spin" /> : <Upload size={15} />}
              {uploading ? t('photos_importing') : t('photos_import')}
            </label>
          )}

          {isAlbums && !activeAlbum && (
            <Button size="sm" icon={<Plus size={15} />} onClick={() => setShowCreateAlbum(true)}>
              {t('new_album')}
            </Button>
          )}
        </div>
      </div>

      {/* Contenu */}
      <div className="flex-1 overflow-y-auto p-6">
        {/* Vue Albums */}
        {isAlbums && !activeAlbum && (
          albumsQuery.isLoading ? (
            <div className="flex items-center justify-center h-48"><Loader2 size={24} className="animate-spin text-text-tertiary" /></div>
          ) : albums.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-64 text-center">
              <BookImage size={48} className="text-text-tertiary mb-3" />
              <p className="text-text-secondary font-medium">{t('no_album')}</p>
              <p className="text-sm text-text-tertiary mt-1">{t('create_album_hint')}</p>
              <Button className="mt-4" onClick={() => setShowCreateAlbum(true)}>
                {t('photos_create_album')}
              </Button>
            </div>
          ) : (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
              {albums.map(album => <AlbumCard key={album.id} album={album} onClick={() => setActiveAlbum(album)} />)}
            </div>
          )
        )}

        {/* Vue Photos */}
        {(!isAlbums || activeAlbum) && (
          photosQuery.isLoading ? (
            <div className="flex items-center justify-center h-48"><Loader2 size={24} className="animate-spin text-text-tertiary" /></div>
          ) : photos.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-64 text-center">
              <Image size={48} className="text-text-tertiary mb-3" />
              <p className="text-text-secondary font-medium">
                {isTrashed ? t('photos_empty_trash') : isStarred ? t('photos_empty_starred') : activeAlbum ? t('photos_empty_album') : t('photos_empty_photos')}
              </p>
              {!isTrashed && !isStarred && !activeAlbum && (
                <p className="text-sm text-text-tertiary mt-1">{t('import_hint')}</p>
              )}
            </div>
          ) : currentView === 'timeline' && !activeAlbum ? (
            <div className="space-y-8">
              {Object.entries(photosByMonth).map(([month, monthPhotos]) => (
                <div key={month}>
                  <h2 className="text-base font-semibold text-text-primary mb-3 capitalize">{month}</h2>
                  <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-2">
                    {monthPhotos.map(photo => (
                      <PhotoCard
                        key={photo.id}
                        photo={photo}
                        selected={selectedPhotos.has(photo.id)}
                        selectionMode={selectedPhotos.size > 0}
                        onSelect={handleSelect}
                        onOpen={openLightbox}
                        onStar={(id, s) => starMutation.mutate({ id, starred: s })}
                        onTrash={id => trashMutation.mutate(id)}
                      />
                    ))}
                  </div>
                </div>
              ))}
            </div>
          ) : (
            <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-2">
              {photos.map(photo => (
                <div key={photo.id} className="relative group/card">
                  <PhotoCard
                    photo={photo}
                    selected={selectedPhotos.has(photo.id)}
                    selectionMode={selectedPhotos.size > 0}
                    onSelect={handleSelect}
                    onOpen={openLightbox}
                    onStar={(id, s) => starMutation.mutate({ id, starred: s })}
                    onTrash={id => isTrashed ? deleteMutation.mutate(id) : trashMutation.mutate(id)}
                  />
                  {isTrashed && (
                    <button
                      onClick={() => restoreMutation.mutate(photo.id)}
                      className="absolute bottom-2 left-1/2 -translate-x-1/2 px-2 py-0.5 text-xs bg-white/90 rounded shadow hover:bg-white opacity-0 group-hover/card:opacity-100 transition-opacity"
                    >
                      {t('restore')}
                    </button>
                  )}
                </div>
              ))}
            </div>
          )
        )}
      </div>

      {/* Input fichier caché */}
      <input
        id="photos-upload-input"
        ref={fileInputRef}
        type="file"
        accept="image/*"
        multiple
        className="hidden"
        onChange={handleFileInput}
      />

      {/* Lightbox avec éditeur */}
      {lightboxPhoto && (
        <PhotosLightbox
          photo={lightboxPhoto}
          photos={photos}
          onClose={() => setLightboxPhoto(null)}
          onNavigate={setLightboxPhoto}
        />
      )}

      {/* Modal création album */}
      {showCreateAlbum && (
        <CreateAlbumModal
          onClose={() => setShowCreateAlbum(false)}
          onCreate={name => createAlbumMutation.mutate(name)}
        />
      )}

      {confirmState && (
        <ConfirmDialog {...confirmState} onConfirm={handleConfirm} onCancel={handleCancel} />
      )}
    </div>
  )
}
