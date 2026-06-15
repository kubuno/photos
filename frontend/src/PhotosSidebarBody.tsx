import { useNavigate, useLocation } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import {
  Image, BookImage, Heart, Trash2, Clock,
  Archive, Video, MapPin, FileText, Camera,
} from 'lucide-react'
import { SidebarNavItem } from '@kubuno/sdk'

const NAV_ITEMS = [
  { tk: 'nav_explore',   label: 'Explorer',                   icon: <Image className="w-4 h-4 flex-shrink-0" />,     path: '/photos' },
  { tk: 'nav_albums',    label: 'Albums',                     icon: <BookImage className="w-4 h-4 flex-shrink-0" />, path: '/photos/albums' },
  { tk: 'nav_documents', label: 'Documents',                  icon: <FileText className="w-4 h-4 flex-shrink-0" />,  path: '/photos/documents' },
  { tk: 'nav_captures',  label: 'Captures & enregistrements', icon: <Camera className="w-4 h-4 flex-shrink-0" />,    path: '/photos/screenshots' },
  { tk: 'nav_favorites', label: 'Favoris',                    icon: <Heart className="w-4 h-4 flex-shrink-0" />,     path: '/photos/starred' },
  { tk: 'nav_places',    label: 'Lieux',                      icon: <MapPin className="w-4 h-4 flex-shrink-0" />,    path: '/photos/places' },
  { tk: 'nav_videos',    label: 'Vidéos',                     icon: <Video className="w-4 h-4 flex-shrink-0" />,     path: '/photos/videos' },
  { tk: 'nav_recent',    label: 'Récemment ajoutées',         icon: <Clock className="w-4 h-4 flex-shrink-0" />,     path: '/photos/recent' },
  { tk: 'nav_archive',   label: 'Archives',                   icon: <Archive className="w-4 h-4 flex-shrink-0" />,   path: '/photos/archive' },
  { tk: 'nav_trash',     label: 'Corbeille',                  icon: <Trash2 className="w-4 h-4 flex-shrink-0" />,    path: '/photos/trash' },
]

export default function PhotosSidebarBody({ collapsed = false }: { collapsed?: boolean }) {
  const navigate     = useNavigate()
  const { pathname } = useLocation()
  const { t }        = useTranslation('photos')

  return (
    <nav className={`flex-1 overflow-y-auto py-1 space-y-0.5 ${collapsed ? "px-2" : "px-3"}`}>
      {NAV_ITEMS.map(({ tk, label, icon, path }) => (
        <SidebarNavItem collapsed={collapsed}
          key={path}
          label={t(tk, { defaultValue: label })}
          icon={icon}
          active={pathname === path}
          onClick={() => navigate(path)}
        />
      ))}
    </nav>
  )
}
