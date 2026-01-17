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
    // parse request
    let body: ShortenRequest = match req.json().await {
        Ok(b) => b,
        Err(_) => return Response::error("invalid json body", 400),
    };
    
    // validate url
    if !body.url.starts_with("http://") && !body.url.starts_with("https://") {
        return Response::error("url must start with http:// or https://", 400);
    }
    
    // generate short code (6 characters)
    let code = generate_code();
    
    // get kv namespace
    let kv = match ctx.env.kv("URLS") {
        Ok(kv) => kv,
        Err(_) => return Response::error("kv namespace not configured", 500),
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
    let mut headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    
    Ok(Response::ok(json)?.with_headers(headers))
}

/// redirect short url to original
async fn handle_redirect(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let code = match ctx.param("code") {
        Some(c) => c,
        None => return Response::error("missing code", 400),
    };
    
    // get kv namespace
    let kv = match ctx.env.kv("URLS") {
        Ok(kv) => kv,
        Err(_) => return Response::error("kv namespace not configured", 500),
    };
    
    // look up the code
    let entry_json = match kv.get(code).text().await? {
        Some(json) => json,
        None => return Response::error("short url not found", 404),
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
        None => return Response::error("missing code", 400),
    };
    
    // get kv namespace
    let kv = match ctx.env.kv("URLS") {
        Ok(kv) => kv,
        Err(_) => return Response::error("kv namespace not configured", 500),
    };
    
    // look up the code
    let entry_json = match kv.get(code).text().await? {
        Some(json) => json,
        None => return Response::error("short url not found", 404),
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
    
    let mut headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    
    Ok(Response::ok(response.to_string())?.with_headers(headers))
}

/// handle cors preflight
fn handle_cors(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let mut headers = Headers::new();
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Access-Control-Allow-Methods", "POST, GET, OPTIONS")?;
    headers.set("Access-Control-Allow-Headers", "Content-Type")?;
    
    Ok(Response::empty()?.with_headers(headers))
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

// ==============================================================================
// tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_code() {
        let code1 = generate_code();
        let code2 = generate_code();
        
        assert_eq!(code1.len(), 6);
        assert_eq!(code2.len(), 6);
        // codes should be alphanumeric
        assert!(code1.chars().all(|c| c.is_alphanumeric()));
    }
}
