interface PhotosLogoProps {
  size?:      number
  className?: string
  title?:     string
}

/** Photos logo (designer artwork, raster). Served by the host from
 *  `/photos-logo.png`; rendered as a square image so it weighs the same as its
 *  neighbours in the waffle menu. */
export function PhotosLogo({ size = 24, className, title = 'Photos' }: PhotosLogoProps) {
  return (
    <img
      src="/photos-logo.png"
      width={size}
      height={size}
      alt={title}
      className={className}
      style={{ display: 'block', objectFit: 'contain' }}
    />
  )
}

export default PhotosLogo
