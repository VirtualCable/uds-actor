use crate::ws::types::{
    LoginRequest, LoginResponse, LogoffRequest, MessageRequest, Ping, PreConnect, RpcError,
    RpcMessage, ScreenshotRequest, ScreenshotResponse, ScriptExecRequest, ScriptExecResponse,
    UUidRequest, UUidResponse, LogoutRequest, LogRequest
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
    UUidRequest => UUidRequest,
    UUidResponse => UUidResponse,
    LogoffRequest => LogoffRequest,
    PreConnect => PreConnect,
    Error => RpcError,
    Ping => Ping,
    LoginRequest => LoginRequest,
    LogoutRequest => LogoutRequest,
    LogRequest => LogRequest,
    ScreenshotRequest => ScreenshotRequest,
    ScriptExecRequest => ScriptExecRequest,
    MessageRequest => MessageRequest,
}
