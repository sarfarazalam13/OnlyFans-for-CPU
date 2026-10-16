import { useEffect } from 'react'
import {
  Activity, AlertTriangle, Check, ChevronRight, Cpu, Fan, Gauge, Info,
  Laptop, LockKeyhole, Radio, ShieldCheck, Thermometer, WifiOff,
} from 'lucide-react'
import {
  Area, AreaChart, CartesianGrid, Line, LineChart, ResponsiveContainer,
  Tooltip, XAxis, YAxis,
} from 'recharts'
import { useMonitorStore } from './store'
import type { CapabilityStatus, TelemetryFrame } from './types'

const statusLabel: Record<CapabilityStatus, string> = {
  monitor_only: 'Monitor only', controllable: 'Controllable', unsupported: 'Unsupported', unknown: 'Unverified',
}

function reading(frame: TelemetryFrame | undefined, type: 'sensor' | 'fan', id: string) {
  const item = type === 'sensor' ? frame?.sensors.find((entry) => entry.id === id) : frame?.fans.find((entry) => entry.id === id)
  if (!item?.valid) return undefined
  return type === 'sensor' ? (item as TelemetryFrame['sensors'][number]).temperatureC ?? undefined : (item as TelemetryFrame['fans'][number]).rpm ?? undefined
}

function MetricCard({ type, name, id, frame }: { type: 'sensor' | 'fan'; name: string; id: string; frame?: TelemetryFrame }) {
  const value = reading(frame, type, id)
  const unit = type === 'sensor' ? '°C' : 'RPM'
  const Icon = type === 'sensor' ? Thermometer : Fan
  return (
    <article className="metric-card">
      <div className="metric-icon"><Icon size={18} aria-hidden="true" /></div>
      <div className="metric-copy">
        <span className="eyebrow">{name}</span>
        <strong>{value === undefined ? '—' : Math.round(value).toLocaleString()} <small>{value === undefined ? 'UNKNOWN' : unit}</small></strong>
      </div>
      <span className={`reading-dot ${value === undefined ? 'unknown' : ''}`} title={value === undefined ? 'Reading unavailable' : 'Reading valid'} />
    </article>
  )
}

function ConnectionBadge({ state }: { state: ReturnType<typeof useMonitorStore.getState>['connection'] }) {
  const Icon = state === 'live' ? Radio : WifiOff
  return <div className={`connection ${state}`}><Icon size={14} /><span>{state === 'live' ? 'Agent live' : state}</span></div>
}

function ChartTooltip({ active, payload, label }: { active?: boolean; payload?: Array<{ color: string; name: string; value?: number; unit?: string }>; label?: string }) {
  if (!active || !payload?.length) return null
  return <div className="chart-tooltip"><span>{label}</span>{payload.filter((item) => item.value != null).map((item) => <b key={item.name} style={{ color: item.color }}>{item.name}: {Math.round(item.value!)}{item.unit}</b>)}</div>
}

function App() {
  const { start, connection, system, fans, sensors, safety, latest, history, error } = useMonitorStore()
  useEffect(() => start(), [start])
  const cpuName = sensors.find((sensor) => sensor.id === 'cpu-package')?.name ?? 'CPU package'
  const gpuName = sensors.find((sensor) => sensor.id === 'gpu-core')?.name ?? 'GPU core'
  const capability = system?.capabilities[0]
  const profileStatus = capability?.status ?? 'unknown'

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <a className="brand" href="#overview" aria-label="FanForge overview">
          <span className="brand-mark"><Fan size={21} /></span>
          <span>FANFORGE<small>THERMAL CONSOLE</small></span>
        </a>
        <nav aria-label="Main navigation">
          <a className="active" href="#overview"><Activity size={18} />Overview</a>
          <a href="#hardware"><Cpu size={18} />Hardware</a>
          <a href="#safety"><ShieldCheck size={18} />Safety</a>
        </nav>
        <div className="sidebar-foot">
          <div className="locked"><LockKeyhole size={16} /><span>CONTROL LOCKED<small>Read-only milestone</small></span></div>
          <span className="version">CORE v0.1.0 · API v1</span>
        </div>
      </aside>

      <main>
        <header className="topbar">
          <div><span className="breadcrumb">SYSTEMS <ChevronRight size={13} /> {system?.identity.model ?? 'LOCAL AGENT'}</span></div>
          <ConnectionBadge state={connection} />
        </header>

        <div className="content" id="overview">
          <section className="hero">
            <div>
              <span className="section-kicker">THERMAL OVERVIEW</span>
              <h1>Cooling, without guesswork.</h1>
              <p>Live telemetry from a safety-first agent. Hardware writes stay locked until this exact device and BIOS pass recovery validation.</p>
            </div>
            <div className="hero-state">
              <span>ACTIVE PROFILE</span>
              <strong>Firmware automatic</strong>
              <small><Check size={13} /> OEM control retained</small>
            </div>
          </section>

          {error && <div className="notice error"><AlertTriangle size={18} /><div><strong>Agent unavailable</strong><span>{error} Start the Rust service in mock mode to view simulated telemetry.</span></div></div>}

          <section className="metrics" aria-label="Current readings">
            <MetricCard type="sensor" name={cpuName} id="cpu-package" frame={latest} />
            <MetricCard type="sensor" name={gpuName} id="gpu-core" frame={latest} />
            <MetricCard type="fan" name="CPU fan" id="cpu-fan" frame={latest} />
            <MetricCard type="fan" name="GPU fan" id="gpu-fan" frame={latest} />
          </section>

          <section className="chart-grid">
            <article className="panel wide">
              <div className="panel-heading"><div><span className="eyebrow">TEMPERATURE HISTORY</span><h2>Thermal envelope</h2></div><span className="time-window">LAST 60 SECONDS</span></div>
              <div className="chart-wrap">
                <ResponsiveContainer width="100%" height="100%">
                  <AreaChart data={history} margin={{ top: 10, right: 6, left: -22, bottom: 0 }}>
                    <defs>
                      <linearGradient id="cpuFill" x1="0" y1="0" x2="0" y2="1"><stop offset="0%" stopColor="#ff6b35" stopOpacity={0.3}/><stop offset="100%" stopColor="#ff6b35" stopOpacity={0}/></linearGradient>
                      <linearGradient id="gpuFill" x1="0" y1="0" x2="0" y2="1"><stop offset="0%" stopColor="#48cae4" stopOpacity={0.2}/><stop offset="100%" stopColor="#48cae4" stopOpacity={0}/></linearGradient>
                    </defs>
                    <CartesianGrid stroke="#22282b" vertical={false} />
                    <XAxis dataKey="time" tick={{ fill: '#707a7e', fontSize: 10 }} tickLine={false} axisLine={false} minTickGap={32} />
                    <YAxis domain={[20, 100]} tick={{ fill: '#707a7e', fontSize: 10 }} tickLine={false} axisLine={false} />
                    <Tooltip content={<ChartTooltip />} />
                    <Area type="monotone" dataKey="cpuTemp" name="CPU" unit="°C" stroke="#ff6b35" strokeWidth={2} fill="url(#cpuFill)" connectNulls={false} />
                    <Area type="monotone" dataKey="gpuTemp" name="GPU" unit="°C" stroke="#48cae4" strokeWidth={2} fill="url(#gpuFill)" connectNulls={false} />
                  </AreaChart>
                </ResponsiveContainer>
              </div>
              <div className="legend"><span><i className="cpu" />CPU package</span><span><i className="gpu" />GPU core</span><span className="live-sample">1s sampling</span></div>
            </article>

            <article className="panel" id="safety">
              <div className="panel-heading"><div><span className="eyebrow">SAFETY SUPERVISOR</span><h2>Fail-closed state</h2></div><ShieldCheck size={20} className="safe-icon" /></div>
              <div className="safety-list">
                <div><span><span className="status-dot good" />Firmware ownership</span><b>RETAINED</b></div>
                <div><span><span className={`status-dot ${safety?.telemetryFresh ? 'good' : 'muted'}`} />Telemetry health</span><b>{safety?.telemetryFresh ? 'FRESH' : 'UNKNOWN'}</b></div>
                <div><span><span className="status-dot good" />Write path</span><b>DISABLED</b></div>
                <div><span><span className="status-dot muted" />Control lease</span><b>NONE</b></div>
              </div>
              <div className="safety-note"><Info size={15} /><span>Any missing or invalid sensor reading is displayed as unknown. It is never converted to 0.</span></div>
            </article>
          </section>

          <section className="lower-grid" id="hardware">
            <article className="panel device-panel">
              <div className="panel-heading"><div><span className="eyebrow">DETECTED HARDWARE</span><h2>Device identity</h2></div><Laptop size={20} /></div>
              <dl>
                <div><dt>Manufacturer</dt><dd>{system?.identity.manufacturer ?? '—'}</dd></div>
                <div><dt>Model</dt><dd>{system?.identity.model ?? '—'}</dd></div>
                <div><dt>SKU</dt><dd>{system?.identity.sku ?? 'Unknown'}</dd></div>
                <div><dt>BIOS</dt><dd>{system?.identity.biosVersion ?? 'Unknown'}</dd></div>
                <div><dt>Operating system</dt><dd>{system ? `${system.identity.operatingSystem}${system.identity.osBuild ? ` · ${system.identity.osBuild}` : ''}` : '—'}</dd></div>
                <div><dt>Probe</dt><dd>{system?.adapter ?? '—'}</dd></div>
              </dl>
            </article>

            <article className="panel capability-panel">
              <div className="panel-heading"><div><span className="eyebrow">COMPATIBILITY REPORT</span><h2>Capability evidence</h2></div><Gauge size={20} /></div>
              <div className={`capability-status ${profileStatus}`}><span className="status-dot" /><div><small>FAN TELEMETRY</small><strong>{statusLabel[profileStatus]}</strong></div></div>
              <p>{capability?.reason ?? 'Waiting for the local agent to identify this machine.'}</p>
              <div className="evidence-row"><span>Evidence</span><b>{capability?.evidence?.replace('_', ' ') ?? 'None'}</b></div>
              <div className="evidence-row"><span>Automatic restore</span><b>{capability?.restorationSupported ? 'Verified' : 'Not verified'}</b></div>
            </article>
          </section>

          <section className="panel rpm-panel">
            <div className="panel-heading"><div><span className="eyebrow">FAN SPEED HISTORY</span><h2>Tachometer</h2></div><span className="time-window">AUTOMATIC MODE</span></div>
            <div className="rpm-chart">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={history} margin={{ top: 8, right: 8, left: -10, bottom: 0 }}>
                  <CartesianGrid stroke="#22282b" vertical={false} />
                  <XAxis dataKey="time" hide /><YAxis tick={{ fill: '#707a7e', fontSize: 10 }} tickLine={false} axisLine={false} width={52} />
                  <Tooltip content={<ChartTooltip />} />
                  <Line type="monotone" dataKey="cpuRpm" name="CPU fan" unit=" RPM" stroke="#ff6b35" strokeWidth={2} dot={false} connectNulls={false} />
                  <Line type="monotone" dataKey="gpuRpm" name="GPU fan" unit=" RPM" stroke="#48cae4" strokeWidth={2} dot={false} connectNulls={false} />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </section>
        </div>
      </main>
    </div>
  )
}

export default App

