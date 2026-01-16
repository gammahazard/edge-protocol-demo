//! ==============================================================================
//! lib.rs - shared types for edge protocol demo
//! ==============================================================================
//!
//! purpose:
//!     defines common types used across all cloudflare workers in this project.
//!     having a shared crate ensures type consistency and reduces duplication.
//!
//! relationships:
//!     - used by: workers/protocol-parser (ModbusFrame)
//!     - used by: workers/telemetry-voter (TelemetryPacket, VoteResult)
//!     - used by: workers/capability-demo (CapabilityTest)
//!     - used by: dashboard (all types for API responses)
//!
//! design rationale:
//!     instead of defining types in each worker, we share them here.
//!     this mirrors industrial patterns where protocol definitions are
//!     centralized and versioned independently of application logic.
//!
//! ==============================================================================

use serde::{Deserialize, Serialize};

// ==============================================================================
// modbus protocol types
// ==============================================================================

/// parsed modbus rtu frame
/// 
/// modbus rtu format:
/// [device_id: 1 byte][function_code: 1 byte][data: n bytes][crc: 2 bytes]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModbusFrame {
    /// device address (1-247)
    pub device_id: u8,
    /// function code (1=read coils, 3=read holding registers, etc.)
    pub function_code: u8,
    /// payload data (varies by function)
    pub data: Vec<u8>,
    /// whether crc validation passed
    pub crc_valid: bool,
}

/// modbus function codes we support
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ModbusFunction {
    ReadCoils = 0x01,
    ReadDiscreteInputs = 0x02,
    ReadHoldingRegisters = 0x03,
    ReadInputRegisters = 0x04,
    WriteSingleCoil = 0x05,
    WriteSingleRegister = 0x06,
    WriteMultipleCoils = 0x0F,
    WriteMultipleRegisters = 0x10,
}

// ==============================================================================
// telemetry and voting types
// ==============================================================================

/// sensor telemetry packet for 2oo3 voting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryPacket {
    /// unique sensor identifier
    pub sensor_id: String,
    /// measured value
    pub value: f64,
    /// unix timestamp in milliseconds
    pub timestamp_ms: u64,
    /// which edge location processed this
    pub edge_location: Option<String>,
}

/// result of 2oo3 tmr voting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteResult {
    /// consensus value (average of agreeing values)
    pub consensus: f64,
    /// values that were rejected as outliers
    pub rejected: Vec<f64>,
    /// fault tolerance status
    pub fault_status: FaultStatus,
}

/// fault tolerance status after voting
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FaultStatus {
    /// all 3 values agreed (within tolerance)
    AllHealthy,
    /// 2 of 3 agreed, 1 rejected
    OneFaulty,
    /// no consensus possible
    NoConsensus,
}

// ==============================================================================
// capability demo types
// ==============================================================================

/// capability test request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityTest {
    /// which capability to test
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
    /// which capability was tested
    pub capability: CapabilityType,
    /// whether it was allowed
    pub allowed: bool,
    /// result message or error
    pub message: String,
}

// ==============================================================================
// tests
// ==============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modbus_frame_serialization() {
        let frame = ModbusFrame {
            device_id: 1,
            function_code: 3,
            data: vec![0x00, 0x0A],
            crc_valid: true,
        };
        let json = serde_json::to_string(&frame).unwrap();
        assert!(json.contains("\"device_id\":1"));
    }

    #[test]
    fn test_vote_result_fault_status() {
        let result = VoteResult {
            consensus: 23.5,
            rejected: vec![99.9],
            fault_status: FaultStatus::OneFaulty,
        };
        assert_eq!(result.fault_status, FaultStatus::OneFaulty);
    }
}
