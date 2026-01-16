# Architecture Overview

## System Design

This project demonstrates WASM capability-based security on Cloudflare's edge network.

```
┌─────────────────────────────────────────────────────────────────┐
│                    CLOUDFLARE EDGE (300+ locations)              │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │ protocol-parser │  │ telemetry-voter │  │ capability-demo │ │
│  │ /api/parse      │  │ /api/vote       │  │ /api/capability │ │
│  │                 │  │                 │  │                 │ │
│  │ Parses Modbus   │  │ 2oo3 TMR voting │  │ Shows allowed/  │ │
│  │ RTU frames      │  │ for telemetry   │  │ blocked ops     │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Capability Model Comparison

| Capability | Your Pi Host | Cloudflare Workers |
|:-----------|:-------------|:-------------------|
| **Allowed** | `gpio-provider` | `fetch`, `KV` |
| **Blocked** | `attack-surface` | `filesystem`, `sockets` |
| **Enforcement** | Wasmtime linker | V8 isolate |

## File Structure

```
edge-protocol-demo/
├── shared/                  # Common types (ModbusFrame, VoteResult)
├── workers/
│   ├── protocol-parser/     # Modbus parsing worker
│   ├── telemetry-voter/     # 2oo3 TMR worker
│   └── capability-demo/     # Security demo worker
├── dashboard/               # Public web UI
└── docs/                    # This documentation
```

## Request Flow

1. User visits dashboard
2. Dashboard calls worker APIs
3. Workers execute Rust WASM on nearest edge node
4. Response returns with edge location header
