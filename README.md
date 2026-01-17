# Edge Protocol Demo

[![Cloudflare Workers](https://img.shields.io/badge/Cloudflare-Workers-F38020?style=for-the-badge&logo=cloudflare&logoColor=white)](https://workers.cloudflare.com/)
[![Rust](https://img.shields.io/badge/Rust-WASM-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Leptos](https://img.shields.io/badge/Leptos-WASM_UI-6f4e37?style=for-the-badge&logo=webassembly&logoColor=white)](https://leptos.dev/)
[![Live Demo](https://img.shields.io/badge/Live-Demo-22c55e?style=for-the-badge&logo=cloudflare&logoColor=white)](https://edge-protocol-demo.pages.dev/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](LICENSE)
[![Deploy Workers](https://github.com/gammahazard/edge-protocol-demo/actions/workflows/deploy.yml/badge.svg?branch=develop)](https://github.com/gammahazard/edge-protocol-demo/actions/workflows/deploy.yml)

**Production-style Cloudflare Workers + Leptos WASM dashboard demonstrating real-world edge computing patterns.**

**🚀 [Try the Live Demo →](https://edge-protocol-demo.pages.dev/)**

---

## What This Project Demonstrates

This project showcases three core Cloudflare Workers use cases that power production applications worldwide:

### 1. URL Shortener — Workers KV in Action
A full-featured URL shortener using **Workers KV** for persistent storage at the edge.

**Features:**
- **Create short URLs** via REST API
- **Click tracking** with automatic counter increment
- **Statistics endpoint** for analytics
- **301 redirects** handled at edge (<50ms globally)
- **Rate limited** — 10 creates/10min per IP to prevent abuse

**Why it matters:** KV is the backbone of many Cloudflare applications. This demonstrates the read-heavy, eventually-consistent patterns that scale to millions of requests.

---

### 2. Rate Limiter — Edge API Protection
A sliding-window rate limiter protecting APIs from abuse — **before requests reach your origin**.

**Features:**
- **Configurable limits** (10 req/min default)
- **Per-client tracking** via IP or API key
- **Standard headers** (`X-RateLimit-Remaining`, `Retry-After`)
- **TTL-based cleanup** — no manual expiration needed
- **Live countdown timer** in dashboard (client-side, instant reset)
- **Edge location display** (shows which Cloudflare POP handled your request)

**Why it matters:** Rate limiting at the edge is Cloudflare's core value proposition. Malicious traffic is blocked in 300+ locations, never reaching your servers.

---

### 3. Capability Demo — Sandbox Security Model
Demonstrates **what Workers can and cannot do** — the same capability-based security as WASI.

| Capability | Status | Reason |
|:-----------|:-------|:-------|
| `fetch()` | ✅ Allowed | HTTP requests to external APIs |
| `KV Storage` | ✅ Allowed | When bound in config |
| `Filesystem` | ❌ Blocked | Workers have no fs access |
| `Raw Sockets` | ❌ Blocked | Only fetch(), no TCP/UDP |
| `Subprocess` | ❌ Blocked | No exec, no shell |

**Features:**
- Test each capability interactively via the dashboard
- See real error messages when blocked capabilities are attempted
- **Rate limited** — 10 tests/5min per IP

**Why it matters:** This is the same security model as WASI — code only gets capabilities the runtime explicitly grants. Demonstrates understanding of sandboxed execution.

---

## Interactive Dashboard

The dashboard is a **full Leptos WASM application** running on Cloudflare Pages:

- **URL Shortener Tab** — Create URLs, view history table with click stats
- **Rate Limiter Tab** — Test rate limiting with live countdown timer
- **Capabilities Tab** — Explore Workers' security model interactively
- **Mobile Responsive** — Card-based layout adapts to any screen size
- **localStorage Persistence** — Your shortened URLs survive browser refreshes

---

## Tech Stack

| Layer | Technology |
|:------|:-----------|
| **Workers** | Rust → `wasm32-unknown-unknown` → Cloudflare Workers |
| **Dashboard** | Leptos 0.7 + Trunk → Cloudflare Pages |
| **Storage** | Workers KV (edge), localStorage (client) |
| **CI/CD** | GitHub Actions → Wrangler deploy |
| **Branching** | Git Flow (`main` → production, `develop` → preview) |

### Deploy Flow

| Component | Deploys Via | Trigger |
|:----------|:------------|:--------|
| **Dashboard** | Cloudflare Pages | Push to `main` or `develop` (dashboard/dist changes) |
| **Workers** | GitHub Actions | Push to `main` or `develop` (workers/** changes only) |

> **Note:** Dashboard changes (`dashboard/**`) are excluded from worker deploys via `paths-ignore` in the workflow. This prevents unnecessary worker rebuilds when only updating the UI. The dashboard is built locally with `trunk build --release` and committed to `dashboard/dist/`.

---

## Quick Start

```bash
# Clone
git clone https://github.com/gammahazard/edge-protocol-demo.git
cd edge-protocol-demo

# Install Wrangler
npm install -g wrangler
wrangler login

# Deploy a worker
cd workers/url-shortener
wrangler deploy
```

---

## API Reference

### URL Shortener

```bash
# Create short URL
curl -X POST https://url-shortener.your.workers.dev/shorten \
  -H "Content-Type: application/json" \
  -d '{"url": "https://github.com/gammahazard"}'
# → {"code": "abc123", "short_url": "https://.../abc123"}

# Use short URL (redirects)
curl -L https://url-shortener.your.workers.dev/abc123

# Get stats
curl https://url-shortener.your.workers.dev/stats/abc123
# → {"clicks": 42, "original_url": "..."}
```

### Rate Limiter

```bash
# Protected endpoint (10 req/min)
curl https://rate-limiter.your.workers.dev/api/protected
# → {"message": "You have accessed the protected resource!"}
# Headers: X-RateLimit-Remaining: 9

# After 10 requests:
# → 429 Too Many Requests
# Headers: Retry-After: 45
```

### Capability Demo

```bash
# Test allowed capability
curl "https://capability-demo.your.workers.dev/api/capability?test=fetch"
# → {"allowed": true}

# Test blocked capability
curl "https://capability-demo.your.workers.dev/api/capability?test=filesystem"
# → {"allowed": false, "message": "BLOCKED: Workers have no filesystem access"}
```

---

## Project Structure

```
edge-protocol-demo/
├── dashboard/              # Leptos WASM web UI (Cloudflare Pages)
│   ├── src/                # Rust components
│   └── dist/               # Built WASM output
│
├── workers/
│   ├── url-shortener/      # KV-backed URL shortening
│   │   ├── src/lib.rs      # Worker logic + documentation
│   │   └── wrangler.toml   # KV bindings
│   ├── rate-limiter/       # Edge rate limiting
│   │   ├── src/lib.rs      # Sliding window algorithm
│   │   └── wrangler.toml   # Rate config vars
│   └── capability-demo/    # Security model demo
│
├── shared/                 # Common types across workers
├── .github/workflows/      # CI/CD pipeline
└── docs/
    └── ARCHITECTURE.md
```

---

## Related Projects

| Project | Description |
|:--------|:------------|
| [Guardian-One Web Demo](https://github.com/gammahazard/Guardian-one-web-demo) | WASI capability security visualization (Leptos WASM) |
| [Edge WASI Runtime](https://github.com/gammahazard/edge-wasi-runtime) | Raspberry Pi + Wasmtime + Python WASM sandboxing |

These projects demonstrate the same **capability-based security** principles at different layers:
- **This project:** Cloudflare's edge runtime
- **Guardian-One:** Browser-based WASI simulation
- **Edge WASI Runtime:** Native Wasmtime on embedded hardware

---

## License

MIT © 2026
