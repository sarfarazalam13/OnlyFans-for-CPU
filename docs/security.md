# Local security model

- The HTTP/WebSocket server binds to `127.0.0.1` by default and permits only development localhost origins.
- Milestone one exposes read-only routes. Reserved mutation routes always return `unsupported_hardware`.
- Hardware control will live in the separate guard process, not the web-facing agent.
- A future Windows named pipe must use an explicit security descriptor limited to LocalSystem, Administrators, and the intended interactive user. The default descriptor is not acceptable.
- A short-lived lease and heartbeat are required for every non-automatic mode. Expiry restores automatic mode.
- Hardware manifests are data only and cannot carry scripts or authorize arbitrary EC addresses.

