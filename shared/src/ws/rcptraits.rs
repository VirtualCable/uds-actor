use crate::ws::types::{
    LoginResponse, Pong, RpcError, RpcMessage, ScreenshotResponse, ScriptExecResponse, LoginRequest, ScreenshotRequest, ScriptExecRequest,
};

macro_rules! impl_tryfrom {
    ($($variant:ident => $ty:ty),* $(,)?) => {
        $(
            impl TryFrom<RpcMessage> for $ty {
                type Error = ();
                fn try_from(msg: RpcMessage) -> Result<Self, Self::Error> {
                    if let RpcMessage::$variant(inner) = msg {
                        Ok(inner)
                    } else {
                        Err(())
                    }
                }
            }
        )*
    };
}

impl_tryfrom! {
    LoginResponse => LoginResponse,
    ScreenshotResponse => ScreenshotResponse,
    ScriptExecResponse => ScriptExecResponse,
    PingResponse => Pong,
    Error => RpcError,
    LoginRequest => LoginRequest,
    ScreenshotRequest => ScreenshotRequest,
    ScriptExecRequest => ScriptExecRequest,
}
