import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '@kubuno/sdk'
import { Image, Save, ChevronLeft, ExternalLink } from 'lucide-react'
import { Link } from 'react-router-dom'
import { Toggle, Button, Tabs } from '@ui'

type Tab = 'gallery' | 'sharing' | 'about'

interface PhotosSettings {
  'photos.thumbnail_size': number
  'photos.jpeg_quality': number
  'photos.trash_auto_delete_days': number
  'photos.allow_public_sharing': boolean
  'photos.share_link_max_days': number
}

const THUMBNAIL_OPTIONS = [
  { value: 128, label: '128 px', descKey: 'photos_thumb_compact' },
  { value: 256, label: '256 px', descKey: 'photos_thumb_recommended' },
  { value: 512, label: '512 px', descKey: 'photos_thumb_high_quality' },
]

const TRASH_DAYS_OPTIONS = [
  { value: 7,  labelKey: 'photos_days_7' },
  { value: 30, labelKey: 'photos_days_30' },
  { value: 90, labelKey: 'photos_days_90' },
  { value: 0,  labelKey: 'photos_never' },
]

const SHARE_LINK_OPTIONS = [
  { value: 7,   labelKey: 'photos_days_7' },
  { value: 30,  labelKey: 'photos_days_30' },
  { value: 90,  labelKey: 'photos_days_90' },
  { value: 365, labelKey: 'photos_one_year' },
  { value: 0,   labelKey: 'photos_unlimited' },
]

function useSettings() {
  return useQuery({
    queryKey: ['admin-settings'],
    queryFn: () =>
      api.get<{ settings: { key: string; value: unknown }[] }>('/admin/settings').then((r) => {
        const map: Record<string, unknown> = {}
        r.data.settings.forEach((s) => { map[s.key] = s.value })
        return map as unknown as PhotosSettings
      }),
  })
}

function GalleryTab() {
  const { t } = useTranslation('photos')
  const queryClient = useQueryClient()
  const { data: settings } = useSettings()

  const [thumbnailSize, setThumbnailSize] = useState<number | null>(null)
  const [jpegQuality, setJpegQuality] = useState<number | null>(null)
  const [trashDays, setTrashDays] = useState<number | null>(null)

  const currentSize   = thumbnailSize   ?? (settings?.['photos.thumbnail_size']         ?? 256)
  const currentQuality = jpegQuality    ?? (settings?.['photos.jpeg_quality']            ?? 85)
  const currentTrash  = trashDays       ?? (settings?.['photos.trash_auto_delete_days']  ?? 30)
  const isDirty = thumbnailSize !== null || jpegQuality !== null || trashDays !== null

  const save = useMutation({
    mutationFn: (updates: Record<string, unknown>) => api.patch('/admin/settings', updates),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-settings'] })
      setThumbnailSize(null)
      setJpegQuality(null)
      setTrashDays(null)
    },
  })

  function handleSave() {
    const updates: Record<string, unknown> = {}
    if (thumbnailSize  !== null) updates['photos.thumbnail_size']           = thumbnailSize
    if (jpegQuality    !== null) updates['photos.jpeg_quality']             = jpegQuality
    if (trashDays      !== null) updates['photos.trash_auto_delete_days']   = trashDays
    if (Object.keys(updates).length > 0) save.mutate(updates)
  }

  return (
    <div>
      <div className="bg-white rounded-xl border border-border divide-y divide-border">
        {/* Thumbnail size */}
        <div className="p-5">
          <p className="text-sm font-medium text-text-primary mb-1">{t('photos_thumb_size_title')}</p>
          <p className="text-xs text-text-secondary mb-3">
            {t('photos_thumb_size_desc')}
          </p>
          <div className="flex gap-3">
            {THUMBNAIL_OPTIONS.map(opt => (
              <button
                key={opt.value}
                onClick={() => setThumbnailSize(opt.value)}
                className={`flex-1 py-3 rounded-xl border text-center transition-colors ${
                  currentSize === opt.value
                    ? 'border-primary bg-primary-light text-primary'
                    : 'border-border hover:bg-surface-1 text-text-secondary'
                }`}
              >
                <p className="text-sm font-semibold">{opt.label}</p>
                <p className="text-xs mt-0.5 opacity-70">{t(opt.descKey)}</p>
              </button>
            ))}
          </div>
        </div>

        {/* JPEG quality */}
        <div className="p-5">
          <p className="text-sm font-medium text-text-primary mb-1">
            {t('photos_jpeg_quality_title')}
          </p>
          <p className="text-xs text-text-secondary mb-3">
            {t('photos_jpeg_quality_desc')}&nbsp;
            <span className="font-medium">{currentQuality}%</span>
          </p>
          <div className="flex items-center gap-3">
            <span className="text-xs text-text-tertiary w-8">1</span>
            <input
              type="range"
              min={50}
              max={100}
              step={5}
              value={currentQuality}
              onChange={(e) => setJpegQuality(Number(e.target.value))}
              className="flex-1"
            />
            <span className="text-xs text-text-tertiary w-8 text-right">100</span>
            <span className="text-sm font-medium text-text-primary w-12 text-right">
              {currentQuality}%
            </span>
          </div>
        </div>

        {/* Trash auto-delete */}
        <div className="p-5">
          <p className="text-sm font-medium text-text-primary mb-1">
            {t('photos_trash_auto_title')}
          </p>
          <p className="text-xs text-text-secondary mb-3">
            {t('photos_trash_auto_desc')}
          </p>
          <div className="flex flex-wrap gap-2">
            {TRASH_DAYS_OPTIONS.map(opt => (
              <button
                key={opt.value}
                onClick={() => setTrashDays(opt.value)}
                className={`px-4 py-1.5 rounded-full text-sm border transition-colors ${
                  currentTrash === opt.value
                    ? 'border-primary bg-primary-light text-primary font-medium'
                    : 'border-border hover:bg-surface-1 text-text-secondary'
                }`}
              >
                {t(opt.labelKey)}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="mt-4 flex justify-end">
        <Button onClick={handleSave} disabled={!isDirty || save.isPending}>
          <Save size={15} />
          {save.isPending ? t('photos_saving') : t('photos_save')}
        </Button>
      </div>
    </div>
  )
}

function SharingTab() {
  const { t } = useTranslation('photos')
  const queryClient = useQueryClient()
  const { data: settings } = useSettings()

  const [allowPublic, setAllowPublic] = useState<boolean | null>(null)
  const [maxDays, setMaxDays] = useState<number | null>(null)

  const currentAllowPublic = allowPublic ?? (settings?.['photos.allow_public_sharing'] ?? true)
  const currentMaxDays     = maxDays     ?? (settings?.['photos.share_link_max_days']   ?? 30)
  const isDirty = allowPublic !== null || maxDays !== null

  const save = useMutation({
    mutationFn: (updates: Record<string, unknown>) => api.patch('/admin/settings', updates),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['admin-settings'] })
      setAllowPublic(null)
      setMaxDays(null)
    },
  })

  function handleSave() {
    const updates: Record<string, unknown> = {}
    if (allowPublic !== null) updates['photos.allow_public_sharing'] = allowPublic
    if (maxDays     !== null) updates['photos.share_link_max_days']   = maxDays
    if (Object.keys(updates).length > 0) save.mutate(updates)
  }

  return (
    <div>
      <div className="bg-white rounded-xl border border-border divide-y divide-border">
        {/* Public sharing toggle */}
        <div className="p-5 flex items-start justify-between gap-4">
          <div>
            <p className="text-sm font-medium text-text-primary">{t('photos_public_sharing_title')}</p>
            <p className="text-xs text-text-secondary mt-0.5">
              {t('photos_public_sharing_desc')}
            </p>
          </div>
          <Toggle checked={currentAllowPublic} onChange={() => setAllowPublic(!currentAllowPublic)} />
        </div>

        {/* Max share link duration */}
        <div className="p-5">
          <p className="text-sm font-medium text-text-primary mb-1">
            {t('photos_share_max_title')}
          </p>
          <p className="text-xs text-text-secondary mb-3">
            {t('photos_share_max_desc')}
          </p>
          <div className="flex flex-wrap gap-2">
            {SHARE_LINK_OPTIONS.map(opt => (
              <button
                key={opt.value}
                onClick={() => setMaxDays(opt.value)}
                disabled={!currentAllowPublic}
                className={`px-4 py-1.5 rounded-full text-sm border transition-colors ${
                  currentMaxDays === opt.value
                    ? 'border-primary bg-primary-light text-primary font-medium'
                    : 'border-border hover:bg-surface-1 text-text-secondary'
                } disabled:opacity-40 disabled:cursor-not-allowed`}
              >
                {t(opt.labelKey)}
              </button>
            ))}
          </div>
        </div>
      </div>

      <div className="mt-4 flex justify-end">
        <Button onClick={handleSave} disabled={!isDirty || save.isPending}>
          <Save size={15} />
          {save.isPending ? t('photos_saving') : t('photos_save')}
        </Button>
      </div>
    </div>
  )
}

function AboutTab() {
  const { t } = useTranslation('photos')
  return (
    <div className="space-y-4">
      <div className="bg-white rounded-xl border border-border overflow-hidden">
        <div className="flex items-center gap-3 px-5 py-4 border-b border-border bg-surface-1">
          <div className="w-10 h-10 rounded-xl bg-pink-100 flex items-center justify-center shrink-0">
            <Image size={20} className="text-pink-600" />
          </div>
          <div>
            <p className="text-sm font-semibold text-text-primary">Kubuno Photos</p>
            <p className="text-xs text-text-tertiary">v0.1.0 · {t('photos_official_module')}</p>
          </div>
          <span className="ml-auto text-xs font-medium px-2 py-0.5 rounded-full bg-orange-100 text-orange-700">
            Rust
          </span>
        </div>

        <div className="divide-y divide-border">
          <div className="px-5 py-4">
            <p className="text-xs font-semibold text-text-tertiary uppercase tracking-wider mb-2">{t('photos_description_label')}</p>
            <p className="text-sm text-text-secondary leading-relaxed">
              {t('photos_description_text')}
            </p>
          </div>

          <div className="px-5 py-4 grid grid-cols-2 gap-4">
            <div>
              <p className="text-xs font-semibold text-text-tertiary uppercase tracking-wider mb-2">{t('photos_author_label')}</p>
              <p className="text-sm text-text-primary">Kubuno Contributors</p>
            </div>
            <div>
              <p className="text-xs font-semibold text-text-tertiary uppercase tracking-wider mb-2">{t('photos_license_label')}</p>
              <p className="text-sm text-text-primary">AGPL-3.0</p>
            </div>
          </div>

          <div className="px-5 py-4">
            <p className="text-xs font-semibold text-text-tertiary uppercase tracking-wider mb-3">{t('photos_technologies_label')}</p>
            <div className="flex flex-wrap gap-2">
              {['Rust', 'Axum 0.7', 'SQLx 0.8', 'PostgreSQL 16', 'image-rs', 'tokio'].map(t => (
                <span key={t} className="text-xs px-2 py-1 rounded-lg bg-surface-2 text-text-secondary font-mono">{t}</span>
              ))}
            </div>
          </div>

          <div className="px-5 py-4">
            <p className="text-xs font-semibold text-text-tertiary uppercase tracking-wider mb-2">{t('photos_links_label')}</p>
            <a
              href="https://github.com/kubuno/kubuno"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline"
            >
              <ExternalLink size={13} />
              github.com/kubuno/kubuno
            </a>
          </div>
        </div>
      </div>
    </div>
  )
}

const TABS: { id: Tab; labelKey: string }[] = [
  { id: 'gallery',  labelKey: 'photos_tab_gallery' },
  { id: 'sharing',  labelKey: 'photos_tab_sharing' },
  { id: 'about',    labelKey: 'photos_tab_about' },
]

export default function PhotosSettingsPage() {
  const { t } = useTranslation('photos')
  const [tab, setTab] = useState<Tab>('gallery')
  const tabs = TABS.map(tb => ({ id: tb.id, label: t(tb.labelKey) }))

  return (
    <div className="max-w-2xl">
      <div className="flex items-center gap-3 mb-6">
        <Link to="/admin?tab=modules" className="p-1.5 rounded-lg hover:bg-surface-2 text-text-secondary hover:text-text-primary transition-colors">
          <ChevronLeft size={18} />
        </Link>
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 rounded-lg bg-pink-100 flex items-center justify-center">
            <Image size={16} className="text-pink-600" />
          </div>
          <div>
            <h1 className="text-lg font-medium text-text-primary">{t('photos_settings_title')}</h1>
            <p className="text-xs text-text-tertiary">{t('photos_settings_subtitle')}</p>
          </div>
        </div>
      </div>

      <Tabs tabs={tabs} value={tab} onChange={setTab} className="mb-6" />

      {tab === 'gallery' && <GalleryTab />}
      {tab === 'sharing' && <SharingTab />}
      {tab === 'about'   && <AboutTab />}
    </div>
  )
}
