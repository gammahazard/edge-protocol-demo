//! ==============================================================================
//! lib.rs - rate limiter cloudflare worker
//! ==============================================================================
//!
//! purpose:
//!     demonstrates edge-based rate limiting using workers kv.
//!     this is a core cloudflare use case - protecting apis from abuse
//!     at the edge before requests reach origin servers.
//!
//! relationships:
//!     - uses: workers kv namespace "RATES" for storing request counters
//!     - deployed to: cloudflare workers edge network
//!
//! cloudflare features demonstrated:
//!     - workers kv (for distributed rate counters)
//!     - environment variables (RATE_LIMIT, RATE_WINDOW_SECONDS)
//!     - edge compute for api protection
//!     - custom response headers (X-RateLimit-*)
//!
//! algorithm:
//!     sliding window rate limiting using fixed window approximation.
//!     each client (identified by ip or api key) gets a counter in kv.
//!     counter expires after window_seconds using kv ttl.
//!
//! api:
//!     GET /api/protected
//!         headers: X-API-Key: <key> (optional, uses ip if not provided)
//!         response: {"data": "..."} or 429 Too Many Requests
//!
//!     GET /api/status
//!         response: {"requests_remaining": 8, "reset_in_seconds": 45}
//!
//! ==============================================================================

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

#[derive(Debug, Serialize)]
struct ProtectedResponse {
    message: String,
    timestamp: u64,
    edge_location: String,
}

#[derive(Debug, Serialize)]
struct StatusResponse {
    client_id: String,
    requests_made: u32,
    requests_remaining: u32,
    limit: u32,
    reset_in_seconds: u64,
}

#[derive(Debug, Serialize)]
struct RateLimitedResponse {
    error: String,
    retry_after_seconds: u64,
    limit: u32,
}

// ==============================================================================
// worker entry point
// ==============================================================================

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();
    
    router
        // protected endpoint (rate limited)
        .get_async("/api/protected", handle_protected)
        // check rate limit status
        .get_async("/api/status", handle_status)
        // health check (not rate limited)
        .get("/health", |_, _| Response::ok("ok"))
        // cors
        .options("/api/protected", handle_cors)
        .options("/api/status", handle_cors)
        .run(req, env)
        .await
}

// ==============================================================================
// request handlers
// ==============================================================================

/// protected endpoint - applies rate limiting
async fn handle_protected(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // get rate limit config from env
    let limit: u32 = ctx.env.var("RATE_LIMIT")
        .map(|v| v.to_string().parse().unwrap_or(10))
        .unwrap_or(10);
    let window_seconds: u64 = ctx.env.var("RATE_WINDOW_SECONDS")
        .map(|v| v.to_string().parse().unwrap_or(60))
        .unwrap_or(60);
    
    // identify client by api key or ip
    let client_id = get_client_id(&req);
    
    // check/update rate limit
    let (allowed, rate_info) = check_rate_limit(&ctx, &client_id, limit, window_seconds).await?;
    
    // calculate remaining
    let remaining = if rate_info.count >= limit { 0 } else { limit - rate_info.count };
    let now = js_sys::Date::now() as u64 / 1000;
    let reset_in = window_seconds.saturating_sub(now.saturating_sub(rate_info.window_start));
    
    if !allowed {
        // rate limited - return 429
        let response = RateLimitedResponse {
            error: "Too Many Requests".to_string(),
            retry_after_seconds: reset_in,
            limit,
        };
        
        let json = serde_json::to_string(&response).unwrap();
        let headers = Headers::new();
        headers.set("Content-Type", "application/json")?;
        headers.set("X-RateLimit-Limit", &limit.to_string())?;
        headers.set("X-RateLimit-Remaining", "0")?;
        headers.set("X-RateLimit-Reset", &reset_in.to_string())?;
        headers.set("Retry-After", &reset_in.to_string())?;
        headers.set("Access-Control-Allow-Origin", "*")?;
        
        let mut resp = Response::ok(json)?;
        resp = resp.with_status(429);
        return Ok(resp.with_headers(headers));
    }
    
    // allowed - return protected data
    let response = ProtectedResponse {
        message: "You have accessed the protected resource!".to_string(),
        timestamp: now,
        edge_location: get_edge_location(&req),
    };
    
    let json = serde_json::to_string(&response).unwrap();
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("X-RateLimit-Limit", &limit.to_string())?;
    headers.set("X-RateLimit-Remaining", &remaining.to_string())?;
    headers.set("X-RateLimit-Reset", &reset_in.to_string())?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    
    Ok(Response::ok(json)?.with_headers(headers))
}

/// get rate limit status without consuming a request
async fn handle_status(req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let limit: u32 = ctx.env.var("RATE_LIMIT")
        .map(|v| v.to_string().parse().unwrap_or(10))
        .unwrap_or(10);
    let window_seconds: u64 = ctx.env.var("RATE_WINDOW_SECONDS")
        .map(|v| v.to_string().parse().unwrap_or(60))
        .unwrap_or(60);
    
    let client_id = get_client_id(&req);
    let rate_info = get_rate_info(&ctx, &client_id).await?;
    
    let now = js_sys::Date::now() as u64 / 1000;
    let reset_in = window_seconds.saturating_sub(now.saturating_sub(rate_info.window_start));
    let remaining = if rate_info.count >= limit { 0 } else { limit - rate_info.count };
    
    let response = StatusResponse {
        client_id: format!("{}...", &client_id[..8.min(client_id.len())]),
        requests_made: rate_info.count,
        requests_remaining: remaining,
        limit,
        reset_in_seconds: reset_in,
    };
    
    let json = serde_json::to_string(&response).unwrap();
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Cache-Control", "public, max-age=2")?; // Cache for 2 seconds
    
    Ok(Response::ok(json)?.with_headers(headers))
}

fn handle_cors(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Access-Control-Allow-Methods", "GET, OPTIONS")?;
    headers.set("Access-Control-Allow-Headers", "Content-Type, X-API-Key")?;
    headers.set("Access-Control-Expose-Headers", "X-RateLimit-Limit, X-RateLimit-Remaining, X-RateLimit-Reset")?;
    
    Ok(Response::empty()?.with_headers(headers))
}

// ==============================================================================
// rate limiting logic
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
    
    // get current rate info
    let mut rate_info = match kv.get(client_id).text().await? {
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
    kv.put(client_id, json)?
        .expiration_ttl(window_seconds)
        .execute()
        .await?;
    
    Ok((true, rate_info))
}

/// get rate info without incrementing
async fn get_rate_info(ctx: &RouteContext<()>, client_id: &str) -> Result<RateInfo> {
    let kv = ctx.env.kv("RATES")?;
    let now = js_sys::Date::now() as u64 / 1000;
    
    match kv.get(client_id).text().await? {
        Some(json) => {
            let info: RateInfo = serde_json::from_str(&json).unwrap_or(RateInfo {
                count: 0,
                window_start: now,
            });
            Ok(info)
        }
        None => Ok(RateInfo { count: 0, window_start: now }),
    }
}

// ==============================================================================
// helpers
// ==============================================================================

/// get client identifier from api key header or ip address
fn get_client_id(req: &Request) -> String {
    let headers = req.headers();
    
    // prefer api key if provided
    if let Ok(Some(key)) = headers.get("X-API-Key") {
        return format!("key:{}", key);
    }
    
    // fall back to cf-connecting-ip (cloudflare provides this)
    if let Ok(Some(ip)) = headers.get("CF-Connecting-IP") {
        return format!("ip:{}", ip);
    }
    
    // last resort
    "unknown".to_string()
}

/// get cloudflare edge location from headers
fn get_edge_location(req: &Request) -> String {
    let headers = req.headers();
    
    if let Ok(Some(colo)) = headers.get("CF-Ray") {
        // cf-ray format: "abc123-LAX" where LAX is the datacenter
        if let Some(pos) = colo.rfind('-') {
            return colo[pos + 1..].to_string();
        }
    }
    "unknown".to_string()
}

// ==============================================================================
// tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================================================
    // RateInfo serialization tests
    // ===========================================================================
    
    #[test]
    fn test_rate_info_serialization() {
        let info = RateInfo {
            count: 5,
            window_start: 1234567890,
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: RateInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.count, 5);
        assert_eq!(parsed.window_start, 1234567890);
    }
    
    #[test]
    fn test_rate_info_zero_count() {
        let info = RateInfo {
            count: 0,
            window_start: 0,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"count\":0"));
    }
    
    #[test]
    fn test_rate_info_max_count() {
        let info = RateInfo {
            count: u32::MAX,
            window_start: u64::MAX,
        };
        let json = serde_json::to_string(&info).unwrap();
        let parsed: RateInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.count, u32::MAX);
        assert_eq!(parsed.window_start, u64::MAX);
    }
    
    // ===========================================================================
    // Response type tests
    // ===========================================================================
    
    #[test]
    fn test_protected_response_serialization() {
        let resp = ProtectedResponse {
            message: "test".to_string(),
            timestamp: 1234567890,
            edge_location: "LAX".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"message\":\"test\""));
        assert!(json.contains("\"edge_location\":\"LAX\""));
    }
    
    #[test]
    fn test_status_response_serialization() {
        let resp = StatusResponse {
            client_id: "ip:1.2...".to_string(),
            requests_made: 5,
            requests_remaining: 5,
            limit: 10,
            reset_in_seconds: 30,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"requests_remaining\":5"));
        assert!(json.contains("\"limit\":10"));
    }
    
    #[test]
    fn test_rate_limited_response_serialization() {
        let resp = RateLimitedResponse {
            error: "rate limited".to_string(),
            retry_after_seconds: 45,
            limit: 10,
        };
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"retry_after_seconds\":45"));
    }
    
    // ===========================================================================
    // Edge location parsing tests
    // ===========================================================================
    
    #[test]
    fn test_cf_ray_format() {
        // Test the CF-Ray format parsing logic directly
        let cf_ray = "abc123def456-LAX";
        if let Some(pos) = cf_ray.rfind('-') {
            let location = &cf_ray[pos + 1..];
            assert_eq!(location, "LAX");
        }
    }
    
    #[test]
    fn test_cf_ray_different_locations() {
        let test_cases = vec![
            ("abc-SJC", "SJC"),
            ("1234-ORD", "ORD"),
            ("test-value-DFW", "DFW"),
        ];
        for (ray, expected) in test_cases {
            if let Some(pos) = ray.rfind('-') {
                assert_eq!(&ray[pos + 1..], expected);
            }
        }
    }
}
