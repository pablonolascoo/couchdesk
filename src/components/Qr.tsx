import { useEffect, useState } from 'react'
import QRCode from 'qrcode'
import { useT } from '../i18n'

// Código QR en SVG, nítido a cualquier tamaño
export function Qr({ text, size = 220 }: { text: string; size?: number }) {
  const t = useT()
  const [svg, setSvg] = useState('')
  useEffect(() => {
    QRCode.toString(text, { type: 'svg', margin: 1, color: { dark: '#111111', light: '#ffffff' } }).then(setSvg)
  }, [text])
  return (
    <div
      className="qr-box"
      style={{ width: size, height: size }}
      role="img"
      aria-label={t('receive.qrLabel')}
      dangerouslySetInnerHTML={{ __html: svg }}
    />
  )
}
