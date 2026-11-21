import { useEffect, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { call, isDesktopApp } from '../native'

export interface RemoteCursor { id: number; name: string; color: string; x: number; y: number }
interface Status { url: string | null; clients: RemoteCursor[] }

const mock: Status = { url: 'http://192.168.1.40:7777/?t=demo', clients: [] }

// Dirección del mando móvil y móviles conectados (en vivo)
export function useRemote() {
  const [status, setStatus] = useState<Status | null>(null)
  useEffect(() => {
    call<Status>('remote_status', {}, mock).then(setStatus).catch(() => {})
    if (!isDesktopApp) return
    const off = listen<RemoteCursor[]>('remote-cursors', (e) => setStatus((s) => (s ? { ...s, clients: e.payload } : s)))
    return () => { off.then((f) => f()) }
  }, [])
  return status
}
