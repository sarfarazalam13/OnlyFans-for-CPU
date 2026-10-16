import { create } from 'zustand'
import type { ConnectionState, FanInfo, HistoryPoint, SafetyState, SensorInfo, SystemResponse, TelemetryFrame } from './types'

const API = import.meta.env.VITE_API_URL ?? ''
const websocketUrl = () => {
  if (import.meta.env.VITE_WS_URL) return import.meta.env.VITE_WS_URL as string
  const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
  return `${protocol}//${location.host}/api/v1/telemetry`
}

interface MonitorStore {
  connection: ConnectionState
  system?: SystemResponse
  fans: FanInfo[]
  sensors: SensorInfo[]
  safety?: SafetyState
  latest?: TelemetryFrame
  history: HistoryPoint[]
  error?: string
  start: () => () => void
}

export function historyPoint(frame: TelemetryFrame): HistoryPoint {
  const sensor = (id: string) => frame.sensors.find((item) => item.id === id)?.temperatureC ?? undefined
  const fan = (id: string) => frame.fans.find((item) => item.id === id)?.rpm ?? undefined
  return {
    time: new Date(frame.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' }),
    cpuTemp: sensor('cpu-package'), gpuTemp: sensor('gpu-core'), cpuRpm: fan('cpu-fan'), gpuRpm: fan('gpu-fan'),
  }
}

export const useMonitorStore = create<MonitorStore>((set, get) => ({
  connection: 'connecting', fans: [], sensors: [], history: [],
  start: () => {
    let socket: WebSocket | undefined
    let retry: ReturnType<typeof setTimeout> | undefined
    let stopped = false

    const loadInventory = async () => {
      try {
        const [system, fans, sensors, safety] = await Promise.all([
          fetch(`${API}/api/v1/system`).then((r) => r.json()),
          fetch(`${API}/api/v1/fans`).then((r) => r.json()),
          fetch(`${API}/api/v1/sensors`).then((r) => r.json()),
          fetch(`${API}/api/v1/safety`).then((r) => r.json()),
        ])
        set({ system, fans: fans.fans, sensors: sensors.sensors, safety, error: undefined })
      } catch {
        set({ error: 'The local FanForge agent is not responding.' })
      }
    }

    const connect = () => {
      if (stopped) return
      set({ connection: get().latest ? 'reconnecting' : 'connecting' })
      socket = new WebSocket(websocketUrl())
      socket.onopen = () => set({ connection: 'live', error: undefined })
      socket.onmessage = (event) => {
        const latest = JSON.parse(event.data) as TelemetryFrame
        set((state) => ({ latest, history: [...state.history, historyPoint(latest)].slice(-60) }))
      }
      socket.onerror = () => socket?.close()
      socket.onclose = () => {
        if (stopped) return
        set({ connection: 'reconnecting' })
        retry = setTimeout(connect, 2000)
      }
    }

    void loadInventory()
    connect()
    const safetyTimer = setInterval(() => void loadInventory(), 10_000)
    return () => {
      stopped = true
      clearTimeout(retry)
      clearInterval(safetyTimer)
      socket?.close()
      set({ connection: 'offline' })
    }
  },
}))

