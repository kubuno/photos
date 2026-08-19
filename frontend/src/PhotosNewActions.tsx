import type { MenuItem } from '@ui'
import { i18n } from '@kubuno/sdk'
import { Image, FolderPlus } from 'lucide-react'
import { usePhotosStore } from './store'

/**
 * Items for the sidebar "New" button (`shell.new-actions` extension point).
 * Built when the menu opens — fresh labels and store state, no hooks.
 */
export function newActionItems(): MenuItem[] {
  if (!window.location.pathname.startsWith('/photos')) return []

  return [
    {
      type: 'action',
      label: i18n.t('photos:photos_import'),
      icon: <Image size={16} />,
      // Clicks the hidden upload input rendered by PhotosApp directly, without
      // going through the store (was a <label htmlFor="photos-upload-input">).
      onClick: () => { document.getElementById('photos-upload-input')?.click() },
    },
    {
      type: 'action',
      label: i18n.t('photos:new_album'),
      icon: <FolderPlus size={16} />,
      onClick: () => usePhotosStore.getState().triggerCreateAlbum(),
    },
  ]
}
