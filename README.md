# Edge Protocol Demo

[![Cloudflare Workers](https://img.shields.io/badge/Cloudflare-Workers-F38020?style=for-the-badge&logo=cloudflare&logoColor=white)](https://workers.cloudflare.com/)
[![Rust](https://img.shields.io/badge/Rust-WASM-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](LICENSE)

**Production-style Cloudflare Workers demonstrating real-world edge computing patterns.**

---

## What This Project Demonstrates

This project showcases three core Cloudflare Workers use cases that power production applications worldwide:

### 1. URL Shortener — Workers KV in Action
A full-featured URL shortener using **Workers KV** for persistent storage at the edge.

**Features:**
- **Create short URLs** via REST API
- **Click tracking** with automatic counter increment
- **Statistics endpoint** for analytics
- **301 redirects** handled at edge (< 50ms globally)

**Why it matters:** KV is the backbone of many Cloudflare applications. This demonstrates the read-heavy, eventually-consistent patterns that scale to millions of requests.

---

### 2. Rate Limiter — Edge API Protection
A sliding-window rate limiter protecting APIs from abuse — **before requests reach your origin**.

**Features:**
- **Configurable limits** (10 req/min default)
- **Per-client tracking** via IP or API key
- **Standard headers** (`X-RateLimit-Remaining`, `Retry-After`)
- **TTL-based cleanup** — no manual expiration needed

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

**Why it matters:** This is the same security model as WASI — code only gets capabilities the runtime explicitly grants. Demonstrates understanding of sandboxed execution.

---

## Tech Stack

| Layer | Technology |
|:------|:-----------|
| **Language** | Rust |
| **Compile Target** | `wasm32-unknown-unknown` |
| **Runtime** | Cloudflare Workers (V8 + Workers Runtime) |
| **Storage** | Workers KV |
| **CI/CD** | GitHub Actions → Wrangler deploy |
| **Branching** | Git Flow (`main` → production, `develop` → preview) |

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
