use crate::ws::types::{
    LoginResponse, Pong, RpcError, RpcMessage, ScreenshotResponse, ScriptExecResponse,
};

impl TryFrom<RpcMessage> for LoginResponse {
    type Error = ();

    fn try_from(msg: RpcMessage) -> Result<Self, Self::Error> {
        if let RpcMessage::LoginResponse(resp) = msg {
            Ok(resp)
        } else {
            Err(())
        }
    }
}

impl TryFrom<RpcMessage> for ScreenshotResponse {
    type Error = ();

    fn try_from(msg: RpcMessage) -> Result<Self, Self::Error> {
        if let RpcMessage::ScreenshotResponse(resp) = msg {
            Ok(resp)
        } else {
            Err(())
        }
    }
}

impl TryFrom<RpcMessage> for ScriptExecResponse {
    type Error = ();

    fn try_from(msg: RpcMessage) -> Result<Self, Self::Error> {
        if let RpcMessage::ScriptExecResponse(resp) = msg {
            Ok(resp)
        } else {
            Err(())
        }
    }
}

impl TryFrom<RpcMessage> for Pong {
    type Error = ();

    fn try_from(msg: RpcMessage) -> Result<Self, Self::Error> {
        if let RpcMessage::PingResponse(resp) = msg {
            Ok(resp)
        } else {
            Err(())
        }
    }
}

impl TryFrom<RpcMessage> for RpcError {
    type Error = ();

    fn try_from(msg: RpcMessage) -> Result<Self, Self::Error> {
        if let RpcMessage::Error(err) = msg {
            Ok(err)
        } else {
            Err(())
        }
    }
}
