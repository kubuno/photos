import { useLocation } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { Image, FolderPlus } from 'lucide-react'
import { usePhotosStore } from './store'

export default function PhotosNewActions() {
  const location = useLocation()
  const { t }    = useTranslation('photos')
  if (!location.pathname.startsWith('/photos')) return null

  return (
    <>
      {/* label htmlFor cible l'input dans PhotosApp directement, sans passer par le store */}
      <label
        htmlFor="photos-upload-input"
        className="flex items-center gap-3 w-full px-3 py-2 text-sm text-text-primary hover:bg-surface-1 cursor-pointer outline-none"
      >
        <Image size={16} className="text-text-secondary" />
        {t('photos_import')}
      </label>
      <button
        onClick={() => usePhotosStore.getState().triggerCreateAlbum()}
        className="flex items-center gap-3 w-full px-3 py-2 text-sm text-text-primary hover:bg-surface-1 cursor-pointer outline-none"
      >
        <FolderPlus size={16} className="text-text-secondary" />
        {t('new_album')}
      </button>
    </>
  )
}
