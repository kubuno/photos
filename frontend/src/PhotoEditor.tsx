import { useState, useEffect } from 'react'
import { useTranslation } from 'react-i18next'
import {
  RotateCcw, RotateCw, FlipHorizontal, FlipVertical,
  Check, Loader2,
} from 'lucide-react'
import { Button } from '@ui'

export interface TransformState {
  rotate: number    // cumulative degrees CW (0 | 90 | 180 | 270)
  flipH:  boolean
  flipV:  boolean
}

interface Props {
  imageUrl:  string          // authenticated blob URL or data URL
  fileName:  string
  onSave:    (transforms: TransformState) => Promise<void>
  onCancel:  () => void
}

function normalizeAngle(a: number) {
  return ((a % 360) + 360) % 360
}

export default function PhotoEditor({ imageUrl, fileName, onSave, onCancel }: Props) {
  const { t: tr }           = useTranslation('photos')
  const [t, setT]           = useState<TransformState>({ rotate: 0, flipH: false, flipV: false })
  const [saving, setSaving]  = useState(false)
  const [imgReady, setImgReady]   = useState(false)
  const [imgFailed, setImgFailed] = useState(false)

  // Build CSS transform string
  const cssTransform = (() => {
    const parts: string[] = []
    if (t.rotate !== 0) parts.push(`rotate(${t.rotate}deg)`)
    const sx = t.flipH ? -1 : 1
    const sy = t.flipV ? -1 : 1
    if (sx !== 1 || sy !== 1) parts.push(`scale(${sx}, ${sy})`)
    return parts.join(' ') || 'none'
  })()

  // Preload via a JS Image object — bypasses any CSS visibility / overflow-hidden
  // tricks that can silently block onLoad in some browser environments.
  useEffect(() => {
    if (!imageUrl) { setImgFailed(true); return }
    let active = true
    setImgReady(false)
    setImgFailed(false)
    const img = new Image()
    img.onload  = () => { if (active) setImgReady(true) }
    img.onerror = () => { if (active) setImgFailed(true) }
    img.src = imageUrl
    return () => {
      active = false
      img.onload  = null
      img.onerror = null
    }
  }, [imageUrl])

  const rotateLeft  = () => setT(p => ({ ...p, rotate: normalizeAngle(p.rotate - 90) }))
  const rotateRight = () => setT(p => ({ ...p, rotate: normalizeAngle(p.rotate + 90) }))
  const flipH       = () => setT(p => ({ ...p, flipH: !p.flipH }))
  const flipV       = () => setT(p => ({ ...p, flipV: !p.flipV }))

  const hasChanges = t.rotate !== 0 || t.flipH || t.flipV

  const handleSave = async () => {
    if (!hasChanges) { onCancel(); return }
    setSaving(true)
    try { await onSave(t) } finally { setSaving(false) }
  }

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !saving) onCancel()
      if ((e.key === 'Enter' || e.key === 's') && hasChanges && !saving) handleSave()
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [saving, hasChanges])

  return (
    <div className="fixed inset-0 z-[60] bg-black flex flex-col">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 bg-black/80 border-b border-white/10 shrink-0">
        <span className="text-white/80 text-sm font-medium truncate max-w-[50vw]">
          {tr('photos_editor_title', { name: fileName })}
        </span>
        <div className="flex items-center gap-2">
          <button
            onClick={onCancel}
            disabled={saving}
            className="px-4 py-1.5 rounded-md text-sm text-white/60 hover:text-white hover:bg-white/10 transition-colors disabled:opacity-40"
          >
            {tr('common_cancel')}
          </button>
          <Button
            size="sm"
            icon={<Check size={14} />}
            onClick={handleSave}
            disabled={!hasChanges}
            loading={saving}
          >
            {saving ? tr('photos_editor_saving') : tr('photos_editor_apply')}
          </Button>
        </div>
      </div>

      {/* Image preview */}
      <div className="flex-1 flex items-center justify-center overflow-hidden bg-[#111]">
        {!imgReady && !imgFailed && (
          <Loader2 size={28} className="text-white/40 animate-spin" />
        )}
        {imgFailed && (
          <p className="text-white/50 text-sm">{tr('photos_editor_preview_failed')}</p>
        )}
        {imgReady && (
          <img
            src={imageUrl}
            alt={fileName}
            draggable={false}
            style={{
              transform: cssTransform,
              maxWidth: '85vw',
              maxHeight: '80vh',
              transition: 'transform 0.2s ease',
            }}
          />
        )}
      </div>

      {/* Toolbar */}
      <div className="shrink-0 flex items-center justify-center gap-2 py-4 bg-black/80 border-t border-white/10">
        <ToolButton onClick={rotateLeft}  icon={<RotateCcw size={18} />}     label={tr('photos_editor_rotate_left')} />
        <ToolButton onClick={rotateRight} icon={<RotateCw size={18} />}      label={tr('photos_editor_rotate_right')} />
        <div className="w-px h-8 bg-white/20 mx-1" />
        <ToolButton onClick={flipH}       icon={<FlipHorizontal size={18} />} label={tr('photos_editor_flip_h')}  active={t.flipH} />
        <ToolButton onClick={flipV}       icon={<FlipVertical size={18} />}   label={tr('photos_editor_flip_v')}    active={t.flipV} />
        {t.rotate !== 0 && (
          <>
            <div className="w-px h-8 bg-white/20 mx-1" />
            <span className="text-white/40 text-xs font-mono">{t.rotate}°</span>
          </>
        )}
      </div>
    </div>
  )
}

function ToolButton({ onClick, icon, label, active }: {
  onClick: () => void
  icon: React.ReactNode
  label: string
  active?: boolean
}) {
  return (
    <button
      onClick={onClick}
      title={label}
      className={`flex flex-col items-center gap-1.5 px-4 py-2 rounded-xl text-xs transition-colors ${
        active
          ? 'bg-primary/20 text-primary border border-primary/40'
          : 'text-white/70 hover:text-white hover:bg-white/10'
      }`}
    >
      {icon}
      <span className="hidden sm:block">{label.split(' (')[0]}</span>
    </button>
  )
}
