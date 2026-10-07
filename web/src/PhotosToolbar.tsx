import { useNavigate, useLocation } from 'react-router-dom'
import { useTranslation } from 'react-i18next'
import { Image, BookImage, Heart, Trash2 } from 'lucide-react'

const TABS = [
  { path: '/photos',         tk: 'photos_tab_explore', label: 'Explorer',   icon: Image },
  { path: '/photos/albums',  tk: 'photos_tab_albums',  label: 'Albums',     icon: BookImage },
  { path: '/photos/starred', tk: 'photos_tab_starred', label: 'Étoilés',    icon: Heart },
  { path: '/photos/trash',   tk: 'photos_tab_trash',   label: 'Corbeille',  icon: Trash2 },
]

export default function PhotosToolbar() {
  const navigate     = useNavigate()
  const { pathname } = useLocation()
  const { t }        = useTranslation('photos')

  // overflow-x-auto : les onglets défilent sur mobile au lieu de déborder
  // (desktop : tout tient, pas de scroll).
  return (
    <div className="flex items-center h-14 px-4 gap-0.5 overflow-x-auto">
      {TABS.map(({ path, tk, label, icon: Icon }) => {
        const active = pathname === path
        return (
          <button
            key={path}
            onClick={() => navigate(path)}
            className={`
              flex items-center gap-2 px-4 h-8 rounded-full text-sm transition-colors font-medium flex-shrink-0 whitespace-nowrap
              ${active
                ? 'bg-primary-light text-primary'
                : 'text-text-secondary hover:bg-surface-2 hover:text-text-primary'}
            `}
          >
            <Icon size={15} />
            {t(tk, { defaultValue: label })}
          </button>
        )
      })}
    </div>
  )
}
