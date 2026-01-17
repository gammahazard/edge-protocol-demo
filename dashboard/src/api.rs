//! ==============================================================================
//! api.rs - API client for calling Cloudflare Workers
//! ==============================================================================

use serde::{Deserialize, Serialize};
use gloo_net::http::Request;

// Base URLs for workers
pub const URL_SHORTENER_BASE: &str = "https://url-shortener-preview.cm-mongo-web3.workers.dev";
pub const RATE_LIMITER_BASE: &str = "https://rate-limiter-preview.cm-mongo-web3.workers.dev";
pub const CAPABILITY_DEMO_BASE: &str = "https://capability-demo-preview.cm-mongo-web3.workers.dev";

// ==============================================================================
// URL Shortener types
// ==============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct ShortenRequest {
    pub url: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ShortenResponse {
    pub code: String,
    pub short_url: String,
    pub original_url: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct UrlStats {
    pub code: String,
    pub original_url: String,
    pub created_at: u64,
    pub clicks: u64,
}

// ==============================================================================
// Rate Limiter types
// ==============================================================================

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitStatus {
    pub client_id: String,
    pub requests_made: u32,
    pub requests_remaining: u32,
    pub limit: u32,
    pub reset_in_seconds: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ProtectedResponse {
    pub message: String,
    pub timestamp: u64,
    pub edge_location: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitedResponse {
    pub error: String,
    pub retry_after_seconds: u64,
    pub limit: u32,
}

// ==============================================================================
// Capability Demo types
// ==============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct CapabilityResult {
    pub capability: String,
    pub allowed: bool,
    pub message: String,
}

// ==============================================================================
// API functions
// ==============================================================================

/// Shorten a URL
pub async fn shorten_url(url: &str) -> Result<ShortenResponse, String> {
    let body = ShortenRequest { url: url.to_string() };
    
    Request::post(&format!("{}/shorten", URL_SHORTENER_BASE))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(&body).unwrap())
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<ShortenResponse>()
        .await
        .map_err(|e| e.to_string())
}

/// Get stats for a short URL
#[allow(dead_code)]
pub async fn get_url_stats(code: &str) -> Result<UrlStats, String> {
    Request::get(&format!("{}/stats/{}", URL_SHORTENER_BASE, code))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<UrlStats>()
        .await
        .map_err(|e| e.to_string())
}

/// Make a request to the protected endpoint
pub async fn test_rate_limit() -> Result<ProtectedResponse, String> {
    let response = Request::get(&format!("{}/api/protected", RATE_LIMITER_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    if response.status() == 429 {
        return Err("Rate limited (429)".to_string());
    }
    
    response
        .json::<ProtectedResponse>()
        .await
        .map_err(|e| e.to_string())
}

/// Get rate limit status
pub async fn get_rate_status() -> Result<RateLimitStatus, String> {
    Request::get(&format!("{}/api/status", RATE_LIMITER_BASE))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<RateLimitStatus>()
        .await
        .map_err(|e| e.to_string())
}

/// Test a capability
pub async fn test_capability(capability: &str) -> Result<CapabilityResult, String> {
    let response = Request::get(&format!("{}/api/capability?test={}", CAPABILITY_DEMO_BASE, capability))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    
    // Check for rate limiting
    if response.status() == 429 {
        return Err("429 - Rate limited".to_string());
    }
    
    response
        .json::<CapabilityResult>()
        .await
        .map_err(|e| e.to_string())
}
