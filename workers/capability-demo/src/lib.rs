//! ==============================================================================
//! lib.rs - capability-based security demo cloudflare worker
//! ==============================================================================
//!
//! purpose:
//!     demonstrates what cloudflare workers CAN and CANNOT do.
//!     shows the same capability-based security model as wasi, but in the
//!     cloudflare context where fetch/kv are allowed but filesystem is blocked.
//!
//! relationships:
//!     - uses: shared (CapabilityTest, CapabilityResult, CapabilityType)
//!     - called by: dashboard (capability explorer tab)
//!     - deployed to: cloudflare workers
//!
//! cloudflare context:
//!     workers run in v8 isolates with no access to:
//!     - filesystem (no fs module)
//!     - raw sockets (only fetch api)
//!     - subprocess/exec
//!     - node.js apis (unless explicitly polyfilled)
//!
//!     this mirrors wasi's capability model where the host decides
//!     what the sandboxed code can access - not the code itself.
//!
//! api:
//!     GET /api/capability?test=fetch
//!     response: { "capability": "Fetch", "allowed": true, "message": "..." }
//!
//!     GET /api/capability?test=filesystem
//!     response: { "capability": "Filesystem", "allowed": false, "message": "..." }
//!
//! security parallel:
//!     cloudflare workers : fetch/kv = your wasi host : gpio-provider
//!     both are capabilities granted by the runtime, not inherent to the code.
//!
//! ==============================================================================

use shared::{CapabilityType, CapabilityResult};
use worker::*;
use serde::{Deserialize, Serialize};

// ==============================================================================
// types
// ==============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct RateInfo {
    count: u32,
    window_start: u64,
}

// ==============================================================================
// worker entry point
// ==============================================================================

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();
    
    router
        .get_async("/api/capability", |req, ctx| handle_capability(req, ctx))
        .get("/api/capabilities", handle_list_capabilities)
        .get("/health", |_, _| Response::ok("ok"))
        .options("/api/capability", handle_cors)
        .run(req, env)
        .await
}

// ==============================================================================
// request handlers
// ==============================================================================

/// handle capability test request
async fn handle_capability(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // check rate limit first
    let limit: u32 = ctx.env.var("RATE_LIMIT")
        .map(|v| v.to_string().parse().unwrap_or(30))
        .unwrap_or(30);
    let window_seconds: u64 = ctx.env.var("RATE_WINDOW_SECONDS")
        .map(|v| v.to_string().parse().unwrap_or(60))
        .unwrap_or(60);
    
    let client_id = get_client_id(&req);
    let (allowed, _) = check_rate_limit(&ctx, &client_id, limit, window_seconds).await?;
    
    if !allowed {
        let headers = Headers::new();
        headers.set("Content-Type", "application/json")?;
        headers.set("Access-Control-Allow-Origin", "*")?;
        let mut resp = Response::ok("{\"error\": \"rate limit exceeded - try again later\"}")?;
        resp = resp.with_status(429);
        return Ok(resp.with_headers(headers));
    }
    
    // parse query parameter
    let url = req.url()?;
    let test = url.query_pairs()
        .find(|(k, _)| k == "test")
        .map(|(_, v)| v.to_string())
        .unwrap_or_default();
    
    // map to capability type
    let capability = match test.as_str() {
        "fetch" => CapabilityType::Fetch,
        "kv" | "kv_storage" => CapabilityType::KvStorage,
        "filesystem" | "fs" => CapabilityType::Filesystem,
        "sockets" | "raw_sockets" => CapabilityType::RawSockets,
        "subprocess" | "exec" => CapabilityType::Subprocess,
        _ => return Response::error("unknown capability. use: fetch, kv, filesystem, sockets, subprocess", 400),
    };
    
    // test the capability
    let result = test_capability(capability, &ctx).await;
    
    // return json response
    let json = serde_json::to_string(&result).unwrap();
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    
    Ok(Response::ok(json)?.with_headers(headers))
}

/// list all capabilities and their status
fn handle_list_capabilities(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let capabilities = vec![
        ("fetch", true, "HTTP requests via fetch() API"),
        ("kv_storage", true, "Workers KV key-value storage"),
        ("filesystem", false, "No filesystem access in Workers"),
        ("raw_sockets", false, "No raw socket access - only fetch()"),
        ("subprocess", false, "No subprocess/exec - no shell access"),
    ];
    
    let json = serde_json::to_string(&capabilities).unwrap();
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Cache-Control", "public, max-age=60")?; // Cache for 60 seconds (static data)
    
    Ok(Response::ok(json)?.with_headers(headers))
}

/// handle cors preflight
fn handle_cors(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Access-Control-Allow-Methods", "GET, OPTIONS")?;
    headers.set("Access-Control-Allow-Headers", "Content-Type")?;
    
    Ok(Response::empty()?.with_headers(headers))
}

// ==============================================================================
// capability testing
// ==============================================================================

/// test a specific capability and return the result
async fn test_capability(capability: CapabilityType, _ctx: &RouteContext<()>) -> CapabilityResult {
    match capability {
        CapabilityType::Fetch => test_fetch().await,
        CapabilityType::KvStorage => test_kv().await,
        CapabilityType::Filesystem => test_filesystem(),
        CapabilityType::RawSockets => test_raw_sockets(),
        CapabilityType::Subprocess => test_subprocess(),
    }
}

/// test fetch capability - ALLOWED
async fn test_fetch() -> CapabilityResult {
    // workers CAN make fetch requests
    match Fetch::Url("https://example.com".parse().unwrap())
        .send()
        .await
    {
        Ok(resp) => CapabilityResult {
            capability: CapabilityType::Fetch,
            allowed: true,
            message: format!("fetch() succeeded - status {}", resp.status_code()),
        },
        Err(e) => CapabilityResult {
            capability: CapabilityType::Fetch,
            allowed: true, // capability exists, just failed
            message: format!("fetch() available but request failed: {}", e),
        },
    }
}

/// test kv storage capability - ALLOWED (if configured)
async fn test_kv() -> CapabilityResult {
    // workers CAN use KV if bound in wrangler.toml
    // for demo, we just report it's available as a capability
    CapabilityResult {
        capability: CapabilityType::KvStorage,
        allowed: true,
        message: "Workers KV is available when bound in wrangler.toml".to_string(),
    }
}

/// test filesystem capability - BLOCKED
fn test_filesystem() -> CapabilityResult {
    // workers CANNOT access filesystem
    // there is no fs module, no File api, no path operations
    CapabilityResult {
        capability: CapabilityType::Filesystem,
        allowed: false,
        message: "BLOCKED: Workers have no filesystem access. No fs module, no File API. \
                  This is equivalent to WASI not granting wasi:filesystem capability.".to_string(),
    }
}

/// test raw sockets capability - BLOCKED
fn test_raw_sockets() -> CapabilityResult {
    // workers CANNOT open raw sockets
    // only fetch() for http/https, connect() for limited tcp
    CapabilityResult {
        capability: CapabilityType::RawSockets,
        allowed: false,
        message: "BLOCKED: Workers cannot open raw sockets. Only fetch() for HTTP \
                  and connect() for limited TCP (WebSocket upgrades). \
                  This is equivalent to WASI not granting wasi:sockets capability.".to_string(),
    }
}

/// test subprocess capability - BLOCKED
fn test_subprocess() -> CapabilityResult {
    // workers CANNOT spawn subprocesses
    // no child_process, no exec, no shell
    CapabilityResult {
        capability: CapabilityType::Subprocess,
        allowed: false,
        message: "BLOCKED: Workers cannot spawn subprocesses. No exec(), no shell access. \
                  Code runs in pure V8 isolate. This is the same isolation as WASM sandbox.".to_string(),
    }
}

// ==============================================================================
// rate limiting
// ==============================================================================

/// check if request is allowed and update counter
async fn check_rate_limit(
    ctx: &RouteContext<()>,
    client_id: &str,
    limit: u32,
    window_seconds: u64,
) -> Result<(bool, RateInfo)> {
    let kv = ctx.env.kv("RATES")?;
    let now = js_sys::Date::now() as u64 / 1000;
    
    // prefix with worker name to avoid collisions
    let key = format!("capability-demo:{}", client_id);
    
    // get current rate info
    let mut rate_info = match kv.get(&key).text().await? {
        Some(json) => {
            let info: RateInfo = serde_json::from_str(&json).unwrap_or(RateInfo {
                count: 0,
                window_start: now,
            });
            // check if window has expired
            if now - info.window_start >= window_seconds {
                RateInfo { count: 0, window_start: now }
            } else {
                info
            }
        }
        None => RateInfo { count: 0, window_start: now },
    };
    
    // check if over limit
    if rate_info.count >= limit {
        return Ok((false, rate_info));
    }
    
    // increment counter
    rate_info.count += 1;
    
    // store updated info with ttl
    let json = serde_json::to_string(&rate_info).unwrap();
    kv.put(&key, json)?
        .expiration_ttl(window_seconds)
        .execute()
        .await?;
    
    Ok((true, rate_info))
}

/// get client identifier from ip address
fn get_client_id(req: &Request) -> String {
    let headers = req.headers();
    
    // use cf-connecting-ip (cloudflare provides this)
    if let Ok(Some(ip)) = headers.get("CF-Connecting-IP") {
        return format!("ip:{}", ip);
    }
    
    "unknown".to_string()
}

// ==============================================================================
// tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_blocked() {
        let result = test_filesystem();
        assert!(!result.allowed);
        assert!(result.message.contains("BLOCKED"));
    }

    #[test]
    fn test_subprocess_blocked() {
        let result = test_subprocess();
        assert!(!result.allowed);
    }

    #[test]
    fn test_kv_allowed() {
        // kv is allowed as a capability
        let result = futures::executor::block_on(test_kv());
        assert!(result.allowed);
    }
}
