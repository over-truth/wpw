use std::io::{self, Read, Write};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub success: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorInfo {
    pub code: String,
    pub message: String,
}

/// Read a Native Messaging message from stdin.
/// Format: 4-byte LE length + UTF-8 JSON.
pub fn read_message() -> io::Result<Message> {
    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    
    // Read 4-byte length prefix
    let mut len_buf = [0u8; 4];
    stdin.read_exact(&mut len_buf)?;
    let len = u32::from_le_bytes(len_buf) as usize;
    
    if len > 1_048_576 {
        // 1MB limit
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Message too large"));
    }
    
    // Read JSON payload
    let mut buf = vec![0u8; len];
    stdin.read_exact(&mut buf)?;
    
    let msg: Message = serde_json::from_slice(&buf)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    
    Ok(msg)
}

/// Write a Native Messaging message to stdout.
pub fn write_message(msg: &Message) -> io::Result<()> {
    let json = serde_json::to_vec(msg)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    
    // Write length prefix
    let len = (json.len() as u32).to_le_bytes();
    stdout.write_all(&len)?;
    stdout.write_all(&json)?;
    stdout.flush()?;
    
    Ok(())
}

impl Message {
    pub fn success(id: Option<String>, payload: serde_json::Value) -> Self {
        Self {
            id,
            msg_type: "response".to_string(),
            payload: Some(payload),
            success: Some(true),
            error: None,
        }
    }
    
    pub fn error(id: Option<String>, code: &str, message: &str) -> Self {
        Self {
            id,
            msg_type: "response".to_string(),
            payload: None,
            success: Some(false),
            error: Some(ErrorInfo {
                code: code.to_string(),
                message: message.to_string(),
            }),
        }
    }
    
    #[allow(dead_code)]
    pub fn event(event_name: &str) -> Self {
        Self {
            id: None,
            msg_type: "event".to_string(),
            payload: Some(serde_json::json!({ "event": event_name })),
            success: None,
            error: None,
        }
    }
}
