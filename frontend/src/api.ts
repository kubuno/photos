import { api as apiClient } from '@kubuno/sdk'

export interface Photo {
  id: string
  owner_id: string
  filename: string
  original_name: string
  mime_type: string
  size_bytes: number
  width: number | null
  height: number | null
  storage_path: string
  content_hash: string | null
  taken_at: string | null
  camera_make: string | null
  camera_model: string | null
  gps_lat: number | null
  gps_lon: number | null
  has_thumbnail: boolean
  has_preview: boolean
  is_starred: boolean
  is_trashed: boolean
  trashed_at: string | null
  description: string | null
  metadata: Record<string, unknown>
  created_at: string
  updated_at: string
}

export interface Album {
  id: string
  owner_id: string
  name: string
  description: string | null
  cover_photo_id: string | null
  photo_count: number
  is_shared: boolean
  share_token: string | null
  created_at: string
  updated_at: string
}

export interface Share {
  id: string
  owner_id: string
  photo_id: string | null
  album_id: string | null
  token: string
  expires_at: string | null
  created_at: string
}

export const photosApi = {
  // Photos
  listPhotos: async (params?: {
    album_id?: string
    starred?: boolean
    trashed?: boolean
    from?: string
    to?: string
    search?: string
    limit?: number
    offset?: number
  }): Promise<{ photos: Photo[] }> => {
    const { data } = await apiClient.get('/photos', { params })
    return data
  },

  uploadPhoto: async (file: File): Promise<{ photo: Photo }> => {
    const form = new FormData()
    form.append('photo', file)
    const { data } = await apiClient.post('/photos', form, {
      headers: { 'Content-Type': 'multipart/form-data' },
    })
    return data
  },

  getPhoto: async (id: string): Promise<{ photo: Photo }> => {
    const { data } = await apiClient.get(`/photos/${id}`)
    return data
  },

  updatePhoto: async (
    id: string,
    dto: { description?: string; is_starred?: boolean; taken_at?: string },
  ): Promise<{ photo: Photo }> => {
    const { data } = await apiClient.patch(`/photos/${id}`, dto)
    return data
  },

  trashPhoto: async (id: string): Promise<void> => {
    await apiClient.post(`/photos/${id}/trash`)
  },

  restorePhoto: async (id: string): Promise<void> => {
    await apiClient.post(`/photos/${id}/restore`)
  },

  deletePhoto: async (id: string): Promise<void> => {
    await apiClient.delete(`/photos/${id}`)
  },

  thumbnailUrl: (photoId: string): string =>
    `/api/v1/photos/${photoId}/thumbnail`,

  previewUrl: (photoId: string): string =>
    `/api/v1/photos/${photoId}/preview`,

  downloadUrl: (photoId: string): string =>
    `/api/v1/photos/${photoId}/download`,

  // Albums
  listAlbums: async (): Promise<{ albums: Album[] }> => {
    const { data } = await apiClient.get('/photos/albums')
    return data
  },

  createAlbum: async (dto: { name: string; description?: string }): Promise<{ album: Album }> => {
    const { data } = await apiClient.post('/photos/albums', dto)
    return data
  },

  getAlbum: async (id: string): Promise<{ album: Album }> => {
    const { data } = await apiClient.get(`/photos/albums/${id}`)
    return data
  },

  updateAlbum: async (
    id: string,
    dto: { name?: string; description?: string; cover_photo_id?: string },
  ): Promise<{ album: Album }> => {
    const { data } = await apiClient.patch(`/photos/albums/${id}`, dto)
    return data
  },

  deleteAlbum: async (id: string): Promise<void> => {
    await apiClient.delete(`/photos/albums/${id}`)
  },

  listAlbumPhotos: async (albumId: string): Promise<{ photos: Photo[] }> => {
    const { data } = await apiClient.get(`/photos/albums/${albumId}/photos`)
    return data
  },

  addPhotosToAlbum: async (albumId: string, photoIds: string[]): Promise<{ added: number }> => {
    const { data } = await apiClient.post(`/photos/albums/${albumId}/photos`, { photo_ids: photoIds })
    return data
  },

  removePhotoFromAlbum: async (albumId: string, photoId: string): Promise<void> => {
    await apiClient.delete(`/photos/albums/${albumId}/photos/${photoId}`)
  },

  // Partages
  listShares: async (): Promise<{ shares: Share[] }> => {
    const { data } = await apiClient.get('/photos/shares')
    return data
  },

  createShare: async (dto: {
    photo_id?: string
    album_id?: string
    expires_in_days?: number
  }): Promise<{ share: Share }> => {
    const { data } = await apiClient.post('/photos/shares', dto)
    return data
  },

  revokeShare: async (id: string): Promise<void> => {
    await apiClient.delete(`/photos/shares/${id}`)
  },
}
