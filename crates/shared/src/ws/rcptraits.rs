use crate::ws::types::{
    Close, LogRequest, LoginRequest, LoginResponse, LogoffRequest, LogoutRequest, MessageRequest,
    Ping, Pong, PreConnect, RpcError, RpcMessage, ScreenshotRequest, ScreenshotResponse,
    ScriptExecRequest, ScriptExecResponse, UUidRequest, UUidResponse,
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

            impl MessageVariant for $ty {
                fn accepts(msg: &RpcMessage) -> bool {
                    matches!(msg, RpcMessage::$variant(_))
                }
            }
        )*
    };
}

/// Variant test without consuming the message: lets `wait_message_arrival`
/// avoid cloning the whole payload just to find out that the receiving worker
/// does not handle it (every subscriber used to clone every message).
pub trait MessageVariant: Sized {
    fn accepts(msg: &RpcMessage) -> bool;
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
    Pong => Pong,
    Close => Close,
    LoginRequest => LoginRequest,
    LogoutRequest => LogoutRequest,
    LogRequest => LogRequest,
    ScreenshotRequest => ScreenshotRequest,
    ScriptExecRequest => ScriptExecRequest,
    MessageRequest => MessageRequest,
}
