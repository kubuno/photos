import { create } from 'zustand'

interface PhotosStore {
  uploadTrigger: number
  createAlbumTrigger: number
  _fileInputClick: (() => void) | null
  triggerUpload: () => void
  triggerCreateAlbum: () => void
  registerFileInput: (fn: () => void) => void
}

export const usePhotosStore = create<PhotosStore>((set, get) => ({
  uploadTrigger: 0,
  createAlbumTrigger: 0,
  _fileInputClick: null,

  triggerUpload: () => {
    const fn = get()._fileInputClick
    if (fn) fn()
    else set(s => ({ uploadTrigger: s.uploadTrigger + 1 }))
  },

  triggerCreateAlbum: () =>
    set(s => ({ createAlbumTrigger: s.createAlbumTrigger + 1 })),

  registerFileInput: (fn) => set({ _fileInputClick: fn }),
}))
