//! ==============================================================================
//! lib.rs - url shortener cloudflare worker
//! ==============================================================================
//!
//! purpose:
//!     a production-style url shortener demonstrating workers kv storage.
//!     this is one of the most common cloudflare workers use cases,
//!     showing real-world patterns for edge key-value operations.
//!
//! relationships:
//!     - uses: shared (ShortenRequest, ShortenResponse types)
//!     - uses: workers kv namespace "URLS" for persistent storage
//!     - deployed to: cloudflare workers edge network
//!
//! cloudflare features demonstrated:
//!     - workers kv (persistent key-value storage)
//!     - json api handling
//!     - http redirects (301)
//!     - cors headers for browser access
//!
//! api:
//!     POST /shorten
//!         body: {"url": "https://example.com/long/path"}
//!         response: {"code": "abc123", "short_url": "https://.../abc123"}
//!
//!     GET /:code
//!         response: 301 redirect to original url
//!
//!     GET /stats/:code
//!         response: {"code": "abc123", "original_url": "...", "clicks": 42}
//!
//! ==============================================================================

use worker::*;
use serde::{Deserialize, Serialize};
use url::Url;

// ==============================================================================
// types
// ==============================================================================

#[derive(Debug, Deserialize)]
struct ShortenRequest {
    url: String,
}

#[derive(Debug, Serialize)]
struct ShortenResponse {
    code: String,
    short_url: String,
    original_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct UrlEntry {
    original_url: String,
    created_at: u64,
    clicks: u64,
}

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
        // shorten a url
        .post_async("/shorten", handle_shorten)
        // get stats for a code
        .get_async("/stats/:code", handle_stats)
        // health check
        .get("/health", |_, _| Response::ok("ok"))
        // cors preflight
        .options("/shorten", handle_cors)
        // redirect short url to original (must be last - catches all)
        .get_async("/:code", handle_redirect)
        .run(req, env)
        .await
}

// ==============================================================================
// request handlers
// ==============================================================================

/// create a short url
async fn handle_shorten(mut req: Request, ctx: RouteContext<()>) -> Result<Response> {
    // check rate limit first
    let limit: u32 = ctx.env.var("RATE_LIMIT")
        .map(|v| v.to_string().parse().unwrap_or(20))
        .unwrap_or(20);
    let window_seconds: u64 = ctx.env.var("RATE_WINDOW_SECONDS")
        .map(|v| v.to_string().parse().unwrap_or(60))
        .unwrap_or(60);
    
    let client_id = get_client_id(&req);
    let (allowed, _) = check_rate_limit(&ctx, &client_id, limit, window_seconds).await?;
    
    if !allowed {
        return cors_error("rate limit exceeded - try again later", 429);
    }
    
    // parse request
    let body: ShortenRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return cors_error("invalid json body", 400),
    };
    
    // validate url (must be valid and use http/https)
    let parsed_url = match Url::parse(&body.url) {
        Ok(u) => u,
        Err(_) => return cors_error("invalid url format", 400),
    };
    
    // only allow http and https schemes
    match parsed_url.scheme() {
        "http" | "https" => {},
        _ => return cors_error("url must use http:// or https://", 400),
    }
    
    // ensure it has a host
    if parsed_url.host_str().is_none() {
        return cors_error("url must have a valid host", 400);
    }
    
    // generate short code (6 characters)
    let code = generate_code();
    
    // get kv namespace
    let kv = match ctx.env.kv("URLS") {
        Ok(kv) => kv,
        Err(_) => return cors_error("kv namespace not configured", 500),
    };
    
    // create entry
    let entry = UrlEntry {
        original_url: body.url.clone(),
        created_at: js_sys::Date::now() as u64,
        clicks: 0,
    };
    
    // store in kv
    let entry_json = serde_json::to_string(&entry).unwrap();
    kv.put(&code, entry_json)?.execute().await?;
    
    // build response
    let host = req.url()?.host_str().unwrap_or("localhost").to_string();
    let protocol = if host.contains("localhost") { "http" } else { "https" };
    
    let response = ShortenResponse {
        code: code.clone(),
        short_url: format!("{}://{}/{}", protocol, host, code),
        original_url: body.url,
    };
    
    // return json with cors
    let json = serde_json::to_string(&response).unwrap();
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    
    Ok(Response::ok(json)?.with_headers(headers))
}

/// redirect short url to original
async fn handle_redirect(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let code = match ctx.param("code") {
        Some(c) => c,
        None => return cors_error("missing code", 400),
    };
    
    // get kv namespace
    let kv = match ctx.env.kv("URLS") {
        Ok(kv) => kv,
        Err(_) => return cors_error("kv namespace not configured", 500),
    };
    
    // look up the code
    let entry_json = match kv.get(code).text().await? {
        Some(json) => json,
        None => return cors_error("short url not found", 404),
    };
    
    // parse entry
    let mut entry: UrlEntry = serde_json::from_str(&entry_json)
        .map_err(|_| Error::from("invalid stored data"))?;
    
    // increment click counter
    entry.clicks += 1;
    let updated_json = serde_json::to_string(&entry).unwrap();
    kv.put(code, updated_json)?.execute().await?;
    
    // redirect to original url
    Response::redirect(entry.original_url.parse()?)
}

/// get stats for a short url
async fn handle_stats(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let code = match ctx.param("code") {
        Some(c) => c,
        None => return cors_error("missing code", 400),
    };
    
    // get kv namespace
    let kv = match ctx.env.kv("URLS") {
        Ok(kv) => kv,
        Err(_) => return cors_error("kv namespace not configured", 500),
    };
    
    // look up the code
    let entry_json = match kv.get(code).text().await? {
        Some(json) => json,
        None => return cors_error("short url not found", 404),
    };
    
    // parse and return
    let entry: UrlEntry = serde_json::from_str(&entry_json)
        .map_err(|_| Error::from("invalid stored data"))?;
    
    let response = serde_json::json!({
        "code": code,
        "original_url": entry.original_url,
        "created_at": entry.created_at,
        "clicks": entry.clicks,
    });
    
    let headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Cache-Control", "public, max-age=5")?; // Cache for 5 seconds
    
    Ok(Response::ok(response.to_string())?.with_headers(headers))
}

/// handle cors preflight
fn handle_cors(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Access-Control-Allow-Methods", "POST, GET, OPTIONS")?;
    headers.set("Access-Control-Allow-Headers", "Content-Type")?;
    
    Ok(Response::empty()?.with_headers(headers))
}

/// helper to create error responses with cors headers
fn cors_error(msg: &str, status: u16) -> Result<Response> {
    let headers = Headers::new();
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Content-Type", "application/json")?;
    
    let body = serde_json::json!({ "error": msg }).to_string();
    
    let mut resp = Response::ok(body)?;
    resp = resp.with_status(status);
    Ok(resp.with_headers(headers))
}

// ==============================================================================
// helpers
// ==============================================================================

/// generate a random 6-character code for short urls
fn generate_code() -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut hasher = DefaultHasher::new();
    js_sys::Date::now().to_bits().hash(&mut hasher);
    js_sys::Math::random().to_bits().hash(&mut hasher);
    
    let hash = hasher.finish();
    
    // convert to base62 (alphanumeric)
    const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut code = String::with_capacity(6);
    let mut n = hash;
    
    for _ in 0..6 {
        code.push(CHARS[(n % 62) as usize] as char);
        n /= 62;
    }
    
    code
}

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
    let key = format!("url-shortener:{}", client_id);
    
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

    // Note: generate_code() uses js_sys APIs which only work in wasm.
    // Those tests are skipped for native test target.
    // The URL validation tests below work on all targets.
    
    // ===========================================================================
    // URL Validation tests (using url crate directly)
    // ===========================================================================
    
    fn validate_url(input: &str) -> Result<Url, &'static str> {
        let parsed = Url::parse(input).map_err(|_| "invalid url format")?;
        match parsed.scheme() {
            "http" | "https" => {},
            _ => return Err("url must use http:// or https://"),
        }
        if parsed.host_str().is_none() {
            return Err("url must have a valid host");
        }
        Ok(parsed)
    }
    
    #[test]
    fn test_valid_http_url() {
        assert!(validate_url("http://example.com").is_ok());
    }
    
    #[test]
    fn test_valid_https_url() {
        assert!(validate_url("https://example.com").is_ok());
    }
    
    #[test]
    fn test_valid_url_with_path() {
        assert!(validate_url("https://example.com/path/to/resource").is_ok());
    }
    
    #[test]
    fn test_valid_url_with_query() {
        assert!(validate_url("https://example.com/search?q=test&page=1").is_ok());
    }
    
    #[test]
    fn test_valid_url_with_port() {
        assert!(validate_url("https://example.com:8080/api").is_ok());
    }
    
    #[test]
    fn test_invalid_url_no_scheme() {
        assert!(validate_url("example.com").is_err());
    }
    
    #[test]
    fn test_invalid_url_wrong_scheme() {
        assert!(validate_url("ftp://example.com").is_err());
        assert!(validate_url("file:///etc/passwd").is_err());
        assert!(validate_url("javascript:alert(1)").is_err());
    }
    
    #[test]
    fn test_invalid_url_empty() {
        assert!(validate_url("").is_err());
    }
    
    #[test]
    fn test_invalid_url_garbage() {
        assert!(validate_url("not a url at all").is_err());
        assert!(validate_url("   ").is_err());
    }
}
