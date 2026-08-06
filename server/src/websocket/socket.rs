use tokio_tungstenite::tungstenite::Message as WsFrame;

use super::{BackendEvent, FrontendCommand};

#[derive(Debug, thiserror::Error)]
pub enum WsError {
    #[error("websocket JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("websocket frame is not text JSON")]
    NonTextFrame,
}

pub fn encode_outgoing(message: &BackendEvent) -> Result<WsFrame, WsError> {
    Ok(WsFrame::Text(serde_json::to_string(message)?.into()))
}

pub fn decode_incoming(frame: &WsFrame) -> Result<Option<FrontendCommand>, WsError> {
    match frame {
        WsFrame::Text(text) => Ok(Some(serde_json::from_str(text)?)),
        WsFrame::Binary(bytes) => Ok(Some(serde_json::from_slice(bytes)?)),
        WsFrame::Ping(_) | WsFrame::Pong(_) | WsFrame::Frame(_) => Ok(None),
        WsFrame::Close(_) => Ok(None),
    }
}
