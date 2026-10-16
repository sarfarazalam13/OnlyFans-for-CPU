import { describe, expect, it } from 'vitest'
import { historyPoint } from './store'

describe('telemetry history', () => {
  it('preserves missing telemetry as undefined instead of zero', () => {
    const point = historyPoint({
      type: 'telemetry', timestamp: '2026-10-16T09:30:00Z', sequence: 1,
      sensors: [{ id: 'cpu-package', temperatureC: null, valid: false, ageMs: 0 }],
      fans: [{ id: 'cpu-fan', rpm: null, valid: false, mode: 'unknown', ageMs: 0 }],
    })
    expect(point.cpuTemp).toBeUndefined()
    expect(point.cpuRpm).toBeUndefined()
  })
})

