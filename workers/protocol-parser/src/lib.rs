//! ==============================================================================
//! lib.rs - modbus protocol parser cloudflare worker
//! ==============================================================================
//!
//! purpose:
//!     parses modbus rtu frames received via http post, validates crc, and
//!     returns structured json. demonstrates industrial protocol handling
//!     on cloudflare's edge network.
//!
//! relationships:
//!     - uses: shared (ModbusFrame, ModbusFunction types)
//!     - called by: dashboard (protocol simulator tab)
//!     - deployed to: cloudflare workers
//!
//! cloudflare context:
//!     this worker runs as wasm on cloudflare's edge nodes (300+ locations).
//!     each request is handled by the nearest edge node, providing low latency
//!     for industrial protocol parsing regardless of client location.
//!
//! api:
//!     POST /api/parse
//!     body: { "frame": "01030000000AC5CD" }  // hex-encoded modbus frame
//!     response: { "device_id": 1, "function_code": 3, "data": [...], "crc_valid": true }
//!
//! ==============================================================================

use shared::{ModbusFrame, ModbusFunction};
use worker::*;

// ==============================================================================
// worker entry point
// ==============================================================================

#[event(fetch)]
async fn fetch(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    // set up router
    let router = Router::new();
    
    router
        // main parsing endpoint
        .post_async("/api/parse", handle_parse)
        // health check
        .get("/health", |_, _| Response::ok("ok"))
        // cors preflight
        .options("/api/parse", handle_cors)
        .run(req, env)
        .await
}

// ==============================================================================
// request handlers
// ==============================================================================

/// handle modbus frame parsing request
async fn handle_parse(mut req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    // parse request body
    let body: serde_json::Value = match req.json().await {
        Ok(v) => v,
        Err(_) => return Response::error("invalid json body", 400),
    };
    
    // extract hex frame
    let frame_hex = match body.get("frame").and_then(|v| v.as_str()) {
        Some(f) => f,
        None => return Response::error("missing 'frame' field", 400),
    };
    
    // parse the modbus frame
    let frame = match parse_modbus_frame(frame_hex) {
        Ok(f) => f,
        Err(e) => return Response::error(format!("parse error: {}", e), 400),
    };
    
    // return json response with cors headers
    let json = serde_json::to_string(&frame).unwrap();
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
// modbus parsing logic
// ==============================================================================

/// parse hex-encoded modbus rtu frame
/// 
/// modbus rtu frame format:
/// [address: 1 byte][function: 1 byte][data: n bytes][crc: 2 bytes]
/// 
/// example: "01030000000AC5CD"
///   address: 0x01 (device 1)
///   function: 0x03 (read holding registers)
///   data: 0x0000000A (start=0, count=10)
///   crc: 0xC5CD
fn parse_modbus_frame(hex: &str) -> std::result::Result<ModbusFrame, String> {
    // decode hex to bytes
    let bytes = hex_decode(hex)?;
    
    if bytes.len() < 4 {
        return Err("frame too short (min 4 bytes)".to_string());
    }
    
    // extract components
    let device_id = bytes[0];
    let function_code = bytes[1];
    let data = bytes[2..bytes.len() - 2].to_vec();
    let frame_crc = u16::from_le_bytes([bytes[bytes.len() - 2], bytes[bytes.len() - 1]]);
    
    // validate crc
    let calculated_crc = calculate_crc16(&bytes[..bytes.len() - 2]);
    let crc_valid = frame_crc == calculated_crc;
    
    Ok(ModbusFrame {
        device_id,
        function_code,
        data,
        crc_valid,
    })
}

/// decode hex string to bytes
fn hex_decode(hex: &str) -> std::result::Result<Vec<u8>, String> {
    if hex.len() % 2 != 0 {
        return Err("hex string must have even length".to_string());
    }
    
    (0..hex.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&hex[i..i + 2], 16)
                .map_err(|_| format!("invalid hex at position {}", i))
        })
        .collect()
}

/// calculate modbus crc-16
/// 
/// polynomial: 0xA001 (reversed 0x8005)
/// initial value: 0xFFFF
fn calculate_crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    
    for byte in data {
        crc ^= *byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    
    crc
}

// ==============================================================================
// tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_decode() {
        let result = hex_decode("01030000000A").unwrap();
        assert_eq!(result, vec![0x01, 0x03, 0x00, 0x00, 0x00, 0x0A]);
    }

    #[test]
    fn test_crc16_calculation() {
        // known good modbus frame without crc
        let data = hex_decode("01030000000A").unwrap();
        let crc = calculate_crc16(&data);
        // expected crc for this frame
        assert_eq!(crc, 0xCDC5); // little endian: C5 CD
    }

    #[test]
    fn test_parse_valid_frame() {
        let frame = parse_modbus_frame("01030000000AC5CD").unwrap();
        assert_eq!(frame.device_id, 1);
        assert_eq!(frame.function_code, 3);
        assert!(frame.crc_valid);
    }

    #[test]
    fn test_parse_invalid_crc() {
        let frame = parse_modbus_frame("01030000000A0000").unwrap();
        assert!(!frame.crc_valid);
    }
}
