export type ConnectionState = 'connecting' | 'live' | 'reconnecting' | 'offline'
export type CapabilityStatus = 'monitor_only' | 'controllable' | 'unsupported' | 'unknown'

export interface HardwareIdentity {
  manufacturer: string
  family?: string
  model: string
  sku?: string
  biosVendor?: string
  biosVersion?: string
  operatingSystem: string
  osBuild?: string
  fingerprint: string
}

export interface Capability {
  operation: string
  status: CapabilityStatus
  adapter: string
  evidence: 'detected' | 'profile_matched' | 'lab_verified'
  restorationSupported: boolean
  reason: string
}

export interface SystemResponse {
  identity: HardwareIdentity
  adapter: string
  capabilities: Capability[]
  diagnostics: Record<string, string>
}

export interface FanInfo {
  id: string
  name: string
  minRpm?: number
  maxRpm?: number
  canReadRpm: boolean
  canSetSpeed: boolean
  canRestoreAuto: boolean
  capability: Capability
}

export interface SensorInfo {
  id: string
  name: string
  kind: 'cpu' | 'gpu' | 'ambient' | 'other'
  available: boolean
}

export interface TelemetryFrame {
  type: 'telemetry'
  timestamp: string
  sequence: number
  sensors: Array<{ id: string; temperatureC: number | null; valid: boolean; ageMs: number }>
  fans: Array<{ id: string; rpm: number | null; mode: string; valid: boolean; ageMs: number }>
}

export interface HistoryPoint {
  time: string
  cpuTemp?: number
  gpuTemp?: number
  cpuRpm?: number
  gpuRpm?: number
}

export interface SafetyState {
  writesEnabled: boolean
  lease: unknown | null
  telemetryFresh: boolean
  thermalState: string
  activeFault: string | null
  lastRestorationResult: string | null
}

