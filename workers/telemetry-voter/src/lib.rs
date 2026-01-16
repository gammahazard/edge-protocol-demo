//! ==============================================================================
//! lib.rs - 2oo3 tmr voting cloudflare worker
//! ==============================================================================
//!
//! purpose:
//!     implements triple modular redundancy (tmr) voting for sensor telemetry.
//!     takes 3 readings, identifies outliers, and returns consensus value.
//!     demonstrates fault tolerance on cloudflare's edge network.
//!
//! relationships:
//!     - uses: shared (TelemetryPacket, VoteResult, FaultStatus types)
//!     - called by: dashboard (tmr voting demo tab)
//!     - deployed to: cloudflare workers
//!
//! cloudflare context:
//!     this worker runs the same 2oo3 voting logic as the guardian-one demo,
//!     but executes on cloudflare's edge. demonstrates that fault tolerance
//!     patterns work identically whether on raspberry pi or global cdn.
//!
//! api:
//!     POST /api/vote
//!     body: { "readings": [23.5, 23.6, 99.9] }
//!     response: { "consensus": 23.55, "rejected": [99.9], "fault_status": "OneFaulty" }
//!
//! algorithm:
//!     1. sort readings by value
//!     2. calculate pairwise differences
//!     3. if all within tolerance → all healthy, average all
//!     4. if 2 within tolerance → one faulty, average the 2
//!     5. if none within tolerance → no consensus
//!
//! ==============================================================================

use shared::{VoteResult, FaultStatus};
use worker::*;
use serde::{Deserialize, Serialize};

/// tolerance for considering two readings as "matching"
/// values within 1.0 of each other are considered the same source
const TOLERANCE: f64 = 1.0;

// ==============================================================================
// request/response types
// ==============================================================================

#[derive(Debug, Deserialize)]
struct VoteRequest {
    readings: Vec<f64>,
}

// ==============================================================================
// worker entry point
// ==============================================================================

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    let router = Router::new();
    
    router
        .post_async("/api/vote", handle_vote)
        .get("/health", |_, _| Response::ok("ok"))
        .options("/api/vote", handle_cors)
        .run(req, env)
        .await
}

// ==============================================================================
// request handlers
// ==============================================================================

/// handle 2oo3 voting request
async fn handle_vote(mut req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    // parse request
    let body: VoteRequest = match req.json().await {
        Ok(v) => v,
        Err(_) => return Response::error("invalid json body", 400),
    };
    
    // validate we have exactly 3 readings for 2oo3
    if body.readings.len() != 3 {
        return Response::error("exactly 3 readings required for 2oo3 voting", 400);
    }
    
    // perform voting
    let result = vote_2oo3(&body.readings);
    
    // return json response
    let json = serde_json::to_string(&result).unwrap();
    let mut headers = Headers::new();
    headers.set("Content-Type", "application/json")?;
    headers.set("Access-Control-Allow-Origin", "*")?;
    
    Ok(Response::ok(json)?.with_headers(headers))
}

/// handle cors preflight
fn handle_cors(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let mut headers = Headers::new();
    headers.set("Access-Control-Allow-Origin", "*")?;
    headers.set("Access-Control-Allow-Methods", "POST, OPTIONS")?;
    headers.set("Access-Control-Allow-Headers", "Content-Type")?;
    
    Ok(Response::empty()?.with_headers(headers))
}

// ==============================================================================
// 2oo3 voting algorithm
// ==============================================================================

/// perform 2-out-of-3 triple modular redundancy voting
/// 
/// the algorithm:
/// 1. compare all pairs of readings
/// 2. if all 3 are within tolerance → average all 3
/// 3. if 2 are within tolerance → average those 2, reject the outlier
/// 4. if none are close → no consensus possible
/// 
/// this is the same algorithm used in industrial safety systems
/// (iec 61508, iec 61511) for critical process control
fn vote_2oo3(readings: &[f64]) -> VoteResult {
    let a = readings[0];
    let b = readings[1];
    let c = readings[2];
    
    // calculate pairwise differences
    let ab_diff = (a - b).abs();
    let bc_diff = (b - c).abs();
    let ac_diff = (a - c).abs();
    
    // check if all three agree
    if ab_diff <= TOLERANCE && bc_diff <= TOLERANCE && ac_diff <= TOLERANCE {
        return VoteResult {
            consensus: (a + b + c) / 3.0,
            rejected: vec![],
            fault_status: FaultStatus::AllHealthy,
        };
    }
    
    // check which pairs agree (2oo3 logic)
    if ab_diff <= TOLERANCE {
        // a and b agree, c is outlier
        VoteResult {
            consensus: (a + b) / 2.0,
            rejected: vec![c],
            fault_status: FaultStatus::OneFaulty,
        }
    } else if bc_diff <= TOLERANCE {
        // b and c agree, a is outlier
        VoteResult {
            consensus: (b + c) / 2.0,
            rejected: vec![a],
            fault_status: FaultStatus::OneFaulty,
        }
    } else if ac_diff <= TOLERANCE {
        // a and c agree, b is outlier
        VoteResult {
            consensus: (a + c) / 2.0,
            rejected: vec![b],
            fault_status: FaultStatus::OneFaulty,
        }
    } else {
        // no pair agrees - byzantine failure
        VoteResult {
            consensus: 0.0,
            rejected: vec![a, b, c],
            fault_status: FaultStatus::NoConsensus,
        }
    }
}

// ==============================================================================
// tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_healthy() {
        let result = vote_2oo3(&[23.5, 23.6, 23.4]);
        assert_eq!(result.fault_status, FaultStatus::AllHealthy);
        assert!(result.rejected.is_empty());
        assert!((result.consensus - 23.5).abs() < 0.1);
    }

    #[test]
    fn test_one_faulty() {
        let result = vote_2oo3(&[23.5, 23.6, 99.9]);
        assert_eq!(result.fault_status, FaultStatus::OneFaulty);
        assert_eq!(result.rejected, vec![99.9]);
        assert!((result.consensus - 23.55).abs() < 0.1);
    }

    #[test]
    fn test_no_consensus() {
        let result = vote_2oo3(&[10.0, 50.0, 90.0]);
        assert_eq!(result.fault_status, FaultStatus::NoConsensus);
        assert_eq!(result.rejected.len(), 3);
    }

    #[test]
    fn test_tolerance_boundary() {
        // exactly at tolerance boundary
        let result = vote_2oo3(&[23.0, 24.0, 23.5]);
        assert_eq!(result.fault_status, FaultStatus::AllHealthy);
    }
}
