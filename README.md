# FanForge

FanForge is a safety-first, open-source laptop monitoring and fan-control platform. The current milestone provides a complete read-only monitoring stack with a deterministic mock adapter and Windows 11 identity discovery for an Acer Nitro 5 AN515-55.

> **Safety:** this repository does not currently write fan modes, fan speeds, or embedded-controller registers. The guard process is a fail-closed protocol skeleton. Control remains unavailable until an exact model, SKU, and BIOS combination passes the documented recovery matrix.

## Run the monitoring demo

Requirements: Node.js 20+ and the stable Rust toolchain.

```powershell
npm install --prefix apps/web
$env:FANFORGE_ADAPTER = "mock"
cargo run -p fanforge-agent
```

In another terminal:

```powershell
npm run dev --prefix apps/web
```

Open `http://localhost:5173`. The API listens on `127.0.0.1:4077` and accepts development origins from localhost only.

On Windows, omit `FANFORGE_ADAPTER=mock` to collect real machine identity. Until a verified sensor provider exists, temperature and RPM readings are reported as unknown rather than fabricated.

## Repository map

- `apps/web` — React monitoring dashboard
- `crates/fanforge-core` — public domain models and hardware traits
- `crates/fanforge-windows` — read-only Windows identity discovery
- `crates/fanforge-safety` — leases, telemetry freshness, thermal faults, and curves
- `crates/fanforge-profiles` — data-only profile validation and SQLite audit storage
- `crates/fanforge-api` — REST and WebSocket transport
- `crates/fanforge-agent` — unprivileged local agent
- `crates/fanforge-guard` — disabled-by-default privileged boundary skeleton
- `contracts/openapi.yaml` — versioned public API contract
- `profiles/windows` — reviewed hardware manifests

See [docs/acer-an515-55-validation.md](docs/acer-an515-55-validation.md) before adding any writable adapter.

