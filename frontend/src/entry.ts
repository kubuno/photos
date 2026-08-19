/**
 * Point d'entrée du bundle MODULE photos (galerie), chargé à l'exécution.
 * Buildé séparément via `vite.module.config.ts` ; specifiers partagés résolus au
 * runtime par l'import map du host. Le host importe ce fichier puis appelle
 * `register()` ; `sdkVersion` permet de rejeter une incompatibilité de contrat.
 */
import { createElement, lazy } from 'react'
import {
  RouteRegistry,
  SlotRegistry,
  ExtensionRegistry,
  ModuleSettingsRegistry,
  WidgetRegistry,
  WaffleAppRegistry,
  useSidebarStore,
  useToolbarStore,
  SDK_VERSION,
  ImageSourceRegistry,
} from '@kubuno/sdk'
import { Image } from 'lucide-react'
import PhotosImageSource from './PhotosImageSource'
import './index.css'
import './i18n'
import PhotosImageViewer from './PhotosImageViewer'
import { newActionItems } from './PhotosNewActions'
import PhotosToolbar from './PhotosToolbar'
import PhotosSidebarBody from './PhotosSidebarBody'
import PhotosRecentWidget from './PhotosRecentWidget'

export const sdkVersion = SDK_VERSION

export function register() {
  WaffleAppRegistry.register('photos', 'Photos', [
    { id: 'photos', label: 'Photos', Icon: Image, path: '/photos' },
  ])

  // The header gear button opens the per-user Photos settings while in /photos.
  ModuleSettingsRegistry.register('photos')

  // Contribute the "Photos" tab to the core image picker (available app-wide).
  ImageSourceRegistry.add({
    id: 'photos', label: 'Photos', order: 10, group: 'library',
    searchable: true, searchPlaceholder: 'Rechercher dans vos photos…',
    icon: createElement(Image, { size: 18 }),
    Component: PhotosImageSource,
  })

  WidgetRegistry.register({ id: 'photos-recent', moduleId: 'photos', Component: PhotosRecentWidget, size: 'medium', order: 30 })

  // When the photos module is active, its image viewer replaces the files module's lightbox.
  SlotRegistry.registerOverride('files-image-viewer', 'photos', PhotosImageViewer)

  useSidebarStore.getState().register({
    moduleId:    'photos',
    routePrefix: '/photos',
    SidebarBody: PhotosSidebarBody,
    collapsedBody: true,
  })

  // Sidebar "New" button: contribute MenuItem[] (data) to the shell menu.
  ExtensionRegistry.register('shell.new-actions', 'photos', {
    moduleId: 'photos',
    items: newActionItems,
  })

  useToolbarStore.getState().register({
    moduleId:         'photos',
    routePrefix:      '/photos',
    ToolbarComponent: PhotosToolbar,
    noPadding:        true,
  })

  useToolbarStore.getState().register({
    moduleId:    'photos-settings',
    routePrefix: '/photos/settings',
  })

  // Routes
  const PhotosApp          = lazy(() => import('./PhotosApp'))
  const PhotosSettingsPage = lazy(() => import('./PhotosSettingsPage'))

  RouteRegistry.register('photos',          PhotosApp)
  RouteRegistry.register('photos/albums',   PhotosApp, { albumsView: true })
  RouteRegistry.register('photos/starred',  PhotosApp, { starred:    true })
  RouteRegistry.register('photos/trash',    PhotosApp, { trashed:    true })
  RouteRegistry.register('photos/settings', PhotosSettingsPage)
}
