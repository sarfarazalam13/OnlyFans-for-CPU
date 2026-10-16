# Acer Nitro 5 AN515-55 validation protocol

This protocol is mandatory before FanForge can mark any AN515-55 profile controllable. Passing on one SKU or BIOS revision does not authorize another.

## Phase A — read-only capture

1. Record exact product name, SKU, BIOS vendor/version, Windows build, CPU, GPU, and installed Acer utilities.
2. Run the agent without administrator rights and export its sanitized discovery report.
3. Record ten minutes of idle telemetry and ten minutes under a repeatable CPU/GPU workload.
4. Compare displayed readings with NitroSense. Missing readings remain unknown.
5. Test reconnect, sleep/resume, logout/login, reboot, and concurrent NitroSense operation.

## Phase B — isolated protocol characterization

This phase happens in a separate experimental branch and is not enabled by a community manifest.

1. Capture the firmware-managed baseline and confirm a reliable readback path.
2. Identify only vendor-defined Auto, Max, and Custom operations; do not scan EC registers.
3. Apply one change at a time while monitoring temperatures, RPM, and system event logs.
4. Restore Auto after every operation, then verify both reported mode and physical RPM response.
5. Repeat after sleep/resume and reboot. Record AC/battery behavior separately.

## Phase C — recovery matrix

A profile stays read-only unless automatic restoration succeeds after every scenario:

- graceful agent shutdown;
- agent crash and heartbeat expiry;
- invalid, missing, or stale sensors;
- thermal emergency;
- user logout;
- sleep/resume;
- guard service restart;
- Windows restart;
- NitroSense launch and conflict;
- malformed or rate-limited API requests.

The final evidence bundle must include the exact identity matcher, BIOS allowlist, before/after readings, event logs, tester identity, date, and a reviewed restoration result. Never include serial numbers or other unique owner data.

