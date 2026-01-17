# Edge Protocol Demo

[![Cloudflare Workers](https://img.shields.io/badge/Cloudflare-Workers-F38020?style=for-the-badge&logo=cloudflare&logoColor=white)](https://workers.cloudflare.com/)
[![Rust](https://img.shields.io/badge/Rust-WASM-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](LICENSE)

**Production-style Cloudflare Workers demonstrating edge computing patterns.**

> URL shortening, rate limiting, and capability-based security — running on Cloudflare's global edge network.

---

## Workers

| Worker | Purpose | Cloudflare Features |
|:-------|:--------|:--------------------|
| **url-shortener** | Shorten and redirect URLs | Workers KV, JSON API |
| **rate-limiter** | Protect APIs from abuse | Workers KV, Rate limiting headers |
| **capability-demo** | Show allowed/blocked operations | Security model |

---

## Quick Start

```bash
# Install Wrangler CLI
npm install -g wrangler

# Login to Cloudflare
wrangler login

# Create KV namespaces
cd workers/url-shortener
wrangler kv namespace create "URLS"
# Copy the ID to wrangler.toml

cd ../rate-limiter
wrangler kv namespace create "RATES"
# Copy the ID to wrangler.toml

# Deploy
wrangler deploy
```

---

## API Examples

### URL Shortener

```bash
# Shorten a URL
curl -X POST https://url-shortener.your.workers.dev/shorten \
  -H "Content-Type: application/json" \
  -d '{"url": "https://example.com/very/long/path"}'

# Response: {"code": "abc123", "short_url": "https://.../abc123"}

# Use the short URL
curl -L https://url-shortener.your.workers.dev/abc123
# Redirects to original URL
```

### Rate Limiter

```bash
# Access protected endpoint (10 requests per minute)
curl https://rate-limiter.your.workers.dev/api/protected

# Check status
curl https://rate-limiter.your.workers.dev/api/status
# Response: {"requests_remaining": 8, "reset_in_seconds": 45}
```

### Capability Demo

```bash
# Test what Workers can do
curl "https://capability-demo.your.workers.dev/api/capability?test=fetch"
# Response: {"allowed": true, "message": "fetch() succeeded"}

curl "https://capability-demo.your.workers.dev/api/capability?test=filesystem"
# Response: {"allowed": false, "message": "BLOCKED: No filesystem access"}
```

---

## Project Structure

```
edge-protocol-demo/
├── workers/
│   ├── url-shortener/       # KV-backed URL shortening
│   ├── rate-limiter/        # Edge rate limiting
│   └── capability-demo/     # Security model demo
├── shared/                  # Common types
├── .github/workflows/       # CI/CD
└── docs/
```

---

## Related Projects

| Project | Focus |
|:--------|:------|
| [Guardian-One Web Demo](https://github.com/gammahazard/Guardian-one-web-demo) | WASI capability security visualization |
| [Edge WASI Runtime](https://github.com/gammahazard/edge-wasi-runtime) | Raspberry Pi + Wasmtime + Python WASM |

---

## License

MIT © 2026
