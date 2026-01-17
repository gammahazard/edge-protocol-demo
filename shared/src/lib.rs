//! ==============================================================================
//! lib.rs - shared types for edge cloudflare workers demo
//! ==============================================================================
//!
//! purpose:
//!     defines common types used across all cloudflare workers in this project.
//!     having a shared crate ensures type consistency and reduces duplication.
//!
//! relationships:
//!     - used by: workers/url-shortener (ShortenRequest, ShortenResponse)
//!     - used by: workers/rate-limiter (RateLimitConfig)
//!     - used by: workers/capability-demo (CapabilityTest, CapabilityResult)
//!
//! design rationale:
//!     centralized types make api changes easier to manage across workers.
//!     for a portfolio project, this demonstrates understanding of rust
//!     workspace patterns and code organization.
//!
//! ==============================================================================

use serde::{Deserialize, Serialize};

// ==============================================================================
// url shortener types
// ==============================================================================

/// request to shorten a url
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortenRequest {
    pub url: String,
}

/// response from shortening a url
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortenResponse {
    pub code: String,
    pub short_url: String,
    pub original_url: String,
}

/// stored url entry in kv
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UrlEntry {
    pub original_url: String,
    pub created_at: u64,
    pub clicks: u64,
}

// ==============================================================================
// rate limiter types
// ==============================================================================

/// rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// maximum requests per window
    pub limit: u32,
    /// window size in seconds
    pub window_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            limit: 10,
            window_seconds: 60,
        }
    }
}

/// rate limit status for a client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitStatus {
    pub client_id: String,
    pub requests_made: u32,
    pub requests_remaining: u32,
    pub limit: u32,
    pub reset_in_seconds: u64,
}

// ==============================================================================
// capability demo types (kept from original)
// ==============================================================================

/// capability test request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityTest {
    pub capability: CapabilityType,
}

/// types of capabilities to test
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CapabilityType {
    /// test fetch() - should be ALLOWED
    Fetch,
    /// test KV storage - should be ALLOWED
    KvStorage,
    /// test filesystem - should be BLOCKED
    Filesystem,
    /// test raw sockets - should be BLOCKED
    RawSockets,
    /// test subprocess - should be BLOCKED
    Subprocess,
}

/// result of capability test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityResult {
    pub capability: CapabilityType,
    pub allowed: bool,
    pub message: String,
}

// ==============================================================================
// tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shorten_request_serialization() {
        let req = ShortenRequest {
            url: "https://example.com".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("example.com"));
    }

    #[test]
    fn test_rate_limit_config_default() {
        let config = RateLimitConfig::default();
        assert_eq!(config.limit, 10);
        assert_eq!(config.window_seconds, 60);
    }
}
