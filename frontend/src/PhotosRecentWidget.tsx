import { useQuery } from '@tanstack/react-query'
import { useTranslation } from 'react-i18next'
import { Image } from 'lucide-react'
import { photosApi } from './api'
import { DashboardWidget } from '@kubuno/sdk'

export default function PhotosRecentWidget() {
  const { t } = useTranslation('photos')
  const { data, isLoading } = useQuery({
    queryKey: ['widget-photos-recent'],
    queryFn:  () => photosApi.listPhotos({ limit: 8 }),
    staleTime: 120_000,
  })

  const photos = data?.photos ?? []

  return (
    <DashboardWidget
      title={t('photos_widget_recent_title')}
      icon={<Image size={15} className="text-pink-500" />}
      link="/photos"
    >
      {isLoading ? (
        <div className="px-4 py-6 text-center text-sm text-text-tertiary">{t('common_loading')}</div>
      ) : photos.length === 0 ? (
        <div className="px-4 py-6 text-center text-sm text-text-tertiary italic">
          {t('photos_widget_recent_empty')}
        </div>
      ) : (
        <div className="grid grid-cols-4 gap-0.5 p-0.5">
          {photos.map(p => (
            <div key={p.id} className="aspect-square bg-surface-2 overflow-hidden">
              {p.has_thumbnail ? (
                <img
                  src={`/api/v1/photos/${p.id}/thumbnail`}
                  alt={p.original_name}
                  className="w-full h-full object-cover hover:scale-105 transition-transform duration-200"
                  loading="lazy"
                />
              ) : (
                <div className="w-full h-full flex items-center justify-center">
                  <Image size={20} className="text-text-tertiary" />
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </DashboardWidget>
  )
}
