import React, { useState } from 'react'
import { Link } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query'
import { api, useAuthStore } from '@kubuno/sdk'
import { Image, Save, ArrowLeft, ExternalLink, Check } from 'lucide-react'
import { Toggle, Button, Radio } from '@ui'
import { useModulePrefs } from './userPrefs'

// ── Per-user preferences (backend, cross-device via core users.preferences) ─────

interface PhotosPrefs {
  density:       string   // 'compact' | 'normal' | 'large'
  sort:          string   // 'date_desc' | 'date_asc' | 'name'
  showFilenames: boolean
  slideshowSecs: string   // '3' | '5' | '10'
  autoplayVideo: boolean
}

const DEFAULT_PREFS: PhotosPrefs = {
  density: 'normal', sort: 'date_desc', showFilenames: false,
  slideshowSecs: '5', autoplayVideo: true,
}

// ── Mail-style layout helpers ───────────────────────────────────────────────────

function SettingsRow({ label, description, children }: {
  label: string; description?: string; children: React.ReactNode
}) {
  return (
    <div className="flex items-start gap-8 py-4 border-b border-[#e8eaed] last:border-0">
      <div className="w-60 flex-shrink-0">
        <p className="text-sm text-[#202124] font-normal">{label}</p>
        {description && <p className="text-xs text-text-tertiary mt-0.5 leading-relaxed">{description}</p>}
      </div>
      <div className="flex-1">{children}</div>
    </div>
  )
}

function RadioGroup({ options, value, onChange }: {
  options: { value: string; label: string }[]; value: string; onChange: (v: string) => void
}) {
  return (
    <div className="flex flex-col items-start gap-2">
      {options.map(opt => (
        <Radio key={opt.value} checked={value === opt.value} onChange={() => onChange(opt.value)} label={opt.label} />
      ))}
    </div>
  )
}

// ── Préférences tab (per-user) ──────────────────────────────────────────────────

function PreferencesTab() {
  const { t } = useTranslation('photos')
  const { prefs: saved, update } = useModulePrefs<PhotosPrefs>('photos', DEFAULT_PREFS)
  const [prefs, setPrefs] = useState<PhotosPrefs>(saved)
  const [savedFlag, setSavedFlag] = useState(false)
  const [busy, setBusy] = useState(false)

  const set = <K extends keyof PhotosPrefs>(key: K, value: PhotosPrefs[K]) =>
    setPrefs(p => ({ ...p, [key]: value }))

  const save = async () => {
    setBusy(true)
    try {
      await update(prefs)
      setSavedFlag(true)
      setTimeout(() => setSavedFlag(false), 2500)
    } finally { setBusy(false) }
  }

  return (
    <div>
      <SettingsRow
        label={t('photos_pref_density', { defaultValue: 'Densité de la grille' })}
        description={t('photos_pref_density_desc', { defaultValue: 'Nombre de photos affichées par ligne.' })}
      >
        <RadioGroup
          value={prefs.density}
          onChange={v => set('density', v)}
          options={[
            { value: 'compact', label: t('photos_pref_density_compact', { defaultValue: 'Compacte (plus de photos)' }) },
            { value: 'normal',  label: t('photos_pref_density_normal',  { defaultValue: 'Normale' }) },
            { value: 'large',   label: t('photos_pref_density_large',   { defaultValue: 'Large (grandes vignettes)' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow label={t('photos_pref_sort', { defaultValue: 'Tri par défaut' })}>
        <RadioGroup
          value={prefs.sort}
          onChange={v => set('sort', v)}
          options={[
            { value: 'date_desc', label: t('photos_pref_sort_recent', { defaultValue: 'Date (plus récentes d\'abord)' }) },
            { value: 'date_asc',  label: t('photos_pref_sort_oldest', { defaultValue: 'Date (plus anciennes d\'abord)' }) },
            { value: 'name',      label: t('photos_pref_sort_name',   { defaultValue: 'Nom du fichier' }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow
        label={t('photos_pref_slideshow', { defaultValue: 'Diaporama' })}
        description={t('photos_pref_slideshow_desc', { defaultValue: 'Durée d\'affichage de chaque photo.' })}
      >
        <RadioGroup
          value={prefs.slideshowSecs}
          onChange={v => set('slideshowSecs', v)}
          options={[
            { value: '3',  label: t('photos_pref_seconds', { defaultValue: '{{count}} secondes', count: 3 }) },
            { value: '5',  label: t('photos_pref_seconds', { defaultValue: '{{count}} secondes', count: 5 }) },
            { value: '10', label: t('photos_pref_seconds', { defaultValue: '{{count}} secondes', count: 10 }) },
          ]}
        />
      </SettingsRow>

      <SettingsRow label={t('photos_pref_filenames', { defaultValue: 'Noms de fichiers' })}>
        <label className="flex items-center gap-2 cursor-pointer select-none">
          <Toggle checked={prefs.showFilenames} onChange={() => set('showFilenames', !prefs.showFilenames)} />
          <span className="text-sm text-text-primary">{t('photos_pref_filenames_on', { defaultValue: 'Afficher les noms sous les vignettes' })}</span>
        </label>
      </SettingsRow>

      <SettingsRow label={t('photos_pref_autoplay', { defaultValue: 'Vidéos' })}>
        <label className="flex items-center gap-2 cursor-pointer select-none">
          <Toggle checked={prefs.autoplayVideo} onChange={() => set('autoplayVideo', !prefs.autoplayVideo)} />
          <span className="text-sm text-text-primary">{t('photos_pref_autoplay_on', { defaultValue: 'Lecture automatique dans la visionneuse' })}</span>
        </label>
      </SettingsRow>

      <div className="pt-5 flex items-center gap-3">
        <Button onClick={save} loading={busy}>
          {savedFlag
            ? <><Check size={14} className="mr-1.5 inline" />{t('photos_settings_saved', { defaultValue: 'Enregistré' })}</>
            : t('photos_settings_save_changes', { defaultValue: 'Enregistrer les modifications' })}
        </Button>
        <Button variant="ghost" onClick={() => setPrefs(saved)}>
          {t('common_cancel', { defaultValue: 'Annuler' })}
        </Button>
      </div>
    </div>
  )
}

// ── Admin-only global settings (instance, via /admin/settings) ──────────────────

interface PhotosSettings {
  'photos.thumbnail_size': number
  'photos.jpeg_quality': number
  'photos.trash_auto_delete_days': number
  'photos.allow_public_sharing': boolean
  'photos.share_link_max_days': number
}

const THUMBNAIL_OPTIONS = [
  { value: 128, label: '128 px' }, { value: 256, label: '256 px' }, { value: 512, label: '512 px' },
]
const TRASH_DAYS_OPTIONS = [
  { value: 7, labelKey: 'photos_days_7' }, { value: 30, labelKey: 'photos_days_30' },
  { value: 90, labelKey: 'photos_days_90' }, { value: 0, labelKey: 'photos_never' },
]
const SHARE_LINK_OPTIONS = [
  { value: 7, labelKey: 'photos_days_7' }, { value: 30, labelKey: 'photos_days_30' },
  { value: 90, labelKey: 'photos_days_90' }, { value: 365, labelKey: 'photos_one_year' },
  { value: 0, labelKey: 'photos_unlimited' },
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
  const [trashDays, setTrashDays] = useState<number | null>(null)

  const currentSize  = thumbnailSize ?? (settings?.['photos.thumbnail_size'] ?? 256)
  const currentTrash = trashDays     ?? (settings?.['photos.trash_auto_delete_days'] ?? 30)
  const isDirty = thumbnailSize !== null || trashDays !== null

  const save = useMutation({
    mutationFn: (updates: Record<string, unknown>) => api.patch('/admin/settings', updates),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['admin-settings'] }); setThumbnailSize(null); setTrashDays(null) },
  })

  function handleSave() {
    const updates: Record<string, unknown> = {}
    if (thumbnailSize !== null) updates['photos.thumbnail_size'] = thumbnailSize
    if (trashDays !== null) updates['photos.trash_auto_delete_days'] = trashDays
    if (Object.keys(updates).length > 0) save.mutate(updates)
  }

  return (
    <div>
      <SettingsRow label={t('photos_thumb_size_title')} description={t('photos_thumb_size_desc')}>
        <div className="flex gap-2">
          {THUMBNAIL_OPTIONS.map(opt => (
            <button key={opt.value} onClick={() => setThumbnailSize(opt.value)}
              className={`px-4 py-1.5 rounded-full text-sm border transition-colors ${
                currentSize === opt.value ? 'border-primary bg-primary-light text-primary font-medium' : 'border-border hover:bg-surface-1 text-text-secondary'}`}>
              {opt.label}
            </button>
          ))}
        </div>
      </SettingsRow>

      <SettingsRow label={t('photos_trash_auto_title')} description={t('photos_trash_auto_desc')}>
        <div className="flex flex-wrap gap-2">
          {TRASH_DAYS_OPTIONS.map(opt => (
            <button key={opt.value} onClick={() => setTrashDays(opt.value)}
              className={`px-4 py-1.5 rounded-full text-sm border transition-colors ${
                currentTrash === opt.value ? 'border-primary bg-primary-light text-primary font-medium' : 'border-border hover:bg-surface-1 text-text-secondary'}`}>
              {t(opt.labelKey)}
            </button>
          ))}
        </div>
      </SettingsRow>

      <div className="pt-5 flex justify-end">
        <Button onClick={handleSave} disabled={!isDirty || save.isPending} icon={<Save size={15} />}>
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
  const [quality, setQuality] = useState<number | null>(null)

  const currentAllowPublic = allowPublic ?? (settings?.['photos.allow_public_sharing'] ?? true)
  const currentMaxDays     = maxDays     ?? (settings?.['photos.share_link_max_days'] ?? 30)
  const currentQuality     = quality     ?? (settings?.['photos.jpeg_quality'] ?? 85)
  const isDirty = allowPublic !== null || maxDays !== null || quality !== null

  const save = useMutation({
    mutationFn: (updates: Record<string, unknown>) => api.patch('/admin/settings', updates),
    onSuccess: () => { queryClient.invalidateQueries({ queryKey: ['admin-settings'] }); setAllowPublic(null); setMaxDays(null); setQuality(null) },
  })

  function handleSave() {
    const updates: Record<string, unknown> = {}
    if (allowPublic !== null) updates['photos.allow_public_sharing'] = allowPublic
    if (maxDays !== null) updates['photos.share_link_max_days'] = maxDays
    if (quality !== null) updates['photos.jpeg_quality'] = quality
    if (Object.keys(updates).length > 0) save.mutate(updates)
  }

  return (
    <div>
      <SettingsRow label={t('photos_public_sharing_title')} description={t('photos_public_sharing_desc')}>
        <Toggle checked={currentAllowPublic} onChange={() => setAllowPublic(!currentAllowPublic)} />
      </SettingsRow>

      <SettingsRow label={t('photos_share_max_title')} description={t('photos_share_max_desc')}>
        <div className="flex flex-wrap gap-2">
          {SHARE_LINK_OPTIONS.map(opt => (
            <button key={opt.value} onClick={() => setMaxDays(opt.value)} disabled={!currentAllowPublic}
              className={`px-4 py-1.5 rounded-full text-sm border transition-colors disabled:opacity-40 disabled:cursor-not-allowed ${
                currentMaxDays === opt.value ? 'border-primary bg-primary-light text-primary font-medium' : 'border-border hover:bg-surface-1 text-text-secondary'}`}>
              {t(opt.labelKey)}
            </button>
          ))}
        </div>
      </SettingsRow>

      <SettingsRow label={t('photos_jpeg_quality_title')} description={t('photos_jpeg_quality_desc')}>
        <div className="flex flex-wrap gap-2">
          {[60, 75, 85, 95].map(q => (
            <button key={q} onClick={() => setQuality(q)}
              className={`px-4 py-1.5 rounded-full text-sm border transition-colors ${
                currentQuality === q ? 'border-primary bg-primary-light text-primary font-medium' : 'border-border hover:bg-surface-1 text-text-secondary'}`}>
              {q}%
            </button>
          ))}
        </div>
      </SettingsRow>

      <div className="pt-5 flex justify-end">
        <Button onClick={handleSave} disabled={!isDirty || save.isPending} icon={<Save size={15} />}>
          {save.isPending ? t('photos_saving') : t('photos_save')}
        </Button>
      </div>
    </div>
  )
}

function AboutTab() {
  const { t } = useTranslation('photos')
  return (
    <div className="rounded-xl border border-border overflow-hidden">
      <div className="flex items-center gap-3 px-5 py-4 border-b border-border bg-surface-1">
        <div className="w-10 h-10 rounded-xl bg-pink-100 flex items-center justify-center shrink-0">
          <Image size={20} className="text-pink-600" />
        </div>
        <div>
          <p className="text-sm font-semibold text-text-primary">Kubuno Photos</p>
          <p className="text-xs text-text-tertiary">v0.1.0 · {t('photos_official_module', { defaultValue: 'Module officiel' })}</p>
        </div>
        <span className="ml-auto text-xs font-medium px-2 py-0.5 rounded-full bg-orange-100 text-orange-700">Rust</span>
      </div>
      <div className="px-5 py-4">
        <a href="https://github.com/kubuno/kubuno" target="_blank" rel="noopener noreferrer"
          className="inline-flex items-center gap-1.5 text-sm text-primary hover:underline">
          <ExternalLink size={13} /> github.com/kubuno/kubuno
        </a>
      </div>
    </div>
  )
}

// ── Main page (mail-style breadcrumb + tab bar) ─────────────────────────────────

type Tab = 'preferences' | 'gallery' | 'sharing' | 'about'

export default function PhotosSettingsPage() {
  const { t } = useTranslation('photos')
  const isAdmin = useAuthStore(s => s.user?.role === 'admin')
  const [tab, setTab] = useState<Tab>('preferences')

  // Admin-only tabs (instance-wide settings) are hidden for non-admins.
  const tabs: { id: Tab; label: string; adminOnly?: boolean }[] = [
    { id: 'preferences', label: t('photos_tab_preferences', { defaultValue: 'Préférences' }) },
    { id: 'gallery',     label: t('photos_tab_gallery', { defaultValue: 'Galerie' }), adminOnly: true },
    { id: 'sharing',     label: t('photos_tab_sharing', { defaultValue: 'Partage' }), adminOnly: true },
    { id: 'about',       label: t('photos_tab_about', { defaultValue: 'À propos' }) },
  ]
  const visibleTabs = tabs.filter(tb => !tb.adminOnly || isAdmin)

  return (
    <div className="flex flex-col h-full bg-white overflow-hidden">
      {/* Breadcrumb header */}
      <div className="flex items-center gap-2 px-6 py-2.5 border-b border-[#e8eaed] flex-shrink-0" style={{ background: '#f8f9fa' }}>
        <Link to="/photos" className="flex items-center gap-1.5 text-sm text-[#1a73e8] hover:underline">
          <ArrowLeft size={14} />
          Photos
        </Link>
        <span className="text-text-tertiary text-sm">/</span>
        <div className="flex items-center gap-1.5">
          <Image size={15} className="text-text-secondary" />
          <span className="text-sm text-text-primary">{t('photos_settings_title', { defaultValue: 'Réglages' })}</span>
        </div>
      </div>

      {/* Tab bar (Gmail-style) */}
      <div className="flex items-end border-b border-[#e8eaed] px-4 flex-shrink-0 overflow-x-auto" style={{ background: '#fff' }}>
        {visibleTabs.map(tb => (
          <button key={tb.id} onClick={() => setTab(tb.id)}
            className={`px-4 py-3 text-sm border-b-2 -mb-px transition-colors whitespace-nowrap ${
              tab === tb.id ? 'border-[#1a73e8] text-[#1a73e8] font-medium' : 'border-transparent text-[#5f6368] hover:text-[#202124] hover:bg-[#f1f3f4]'}`}>
            {tb.label}
          </button>
        ))}
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        <div className="max-w-3xl mx-auto px-8 py-6">
          {tab === 'preferences' && <PreferencesTab />}
          {tab === 'gallery'  && isAdmin && <GalleryTab />}
          {tab === 'sharing'  && isAdmin && <SharingTab />}
          {tab === 'about'    && <AboutTab />}
        </div>
      </div>
    </div>
  )
}
