# Edge Protocol Demo

[![Cloudflare Workers](https://img.shields.io/badge/Cloudflare-Workers-F38020?style=for-the-badge&logo=cloudflare&logoColor=white)](https://workers.cloudflare.com/)
[![Rust](https://img.shields.io/badge/Rust-WASM-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](LICENSE)

**Industrial protocol parsing with 2oo3 TMR voting — running on Cloudflare's global edge network.**

> Demonstrates WASM capability-based security using the same patterns as embedded edge gateways, but deployed to 300+ global data centers.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    PUBLIC DASHBOARD                              │
│                    (Cloudflare Pages)                            │
│  • Protocol simulator   • TMR voting demo   • Capability explorer│
└────────────────────────────────┬────────────────────────────────┘
                                 │
        ┌────────────────────────┼────────────────────────┐
        ▼                        ▼                        ▼
┌───────────────┐      ┌───────────────┐      ┌───────────────┐
│ /api/parse    │      │ /api/vote     │      │ /api/capability│
│ Protocol      │      │ 2oo3 TMR      │      │ Security      │
│ Parser        │      │ Voter         │      │ Demo          │
│ (Rust WASM)   │      │ (Rust WASM)   │      │ (Rust WASM)   │
└───────────────┘      └───────────────┘      └───────────────┘
```

---

## Quick Start

```bash
# Install Wrangler CLI
npm install -g wrangler

# Login to Cloudflare
wrangler login

# Deploy all workers
cd workers/protocol-parser && wrangler deploy
cd ../telemetry-voter && wrangler deploy
cd ../capability-demo && wrangler deploy
```

---

## Project Structure

```
edge-protocol-demo/
├── workers/
│   ├── protocol-parser/     # Modbus/OPC-UA frame parsing
│   ├── telemetry-voter/     # 2oo3 TMR consensus voting
│   └── capability-demo/     # Shows allowed vs blocked operations
├── shared/                  # Common types across workers
├── dashboard/               # Public web UI (Cloudflare Pages)
└── docs/                    # Architecture documentation
```

---

## Related Projects

| Project | Focus | Demo |
|---------|-------|------|
| [Guardian-One Web Demo](https://github.com/gammahazard/Guardian-one-web-demo) | Browser-based attack simulation | [Live](https://guardian-one.vercel.app) |
| [Edge WASI Runtime](https://github.com/gammahazard/edge-wasi-runtime) | Raspberry Pi hardware integration | Video |

---

## License

MIT © 2026
