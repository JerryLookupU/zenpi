#![cfg(feature = "remote")]
//! Stage 2 security target wrapper. The adversarial authentication and ACL
//! coverage lives in the production integration target so it is always built
//! by Cargo's locked test surface.

include!("remote_auth.rs");

#[test]
fn post_dispatch_delivery_refusal_never_claims_a_write_had_no_effect() {
    use zenpi::remote::{
        frame::{read_frame, write_frame},
        host_adapter::HostAdapter,
        protocol::{ErrorCode, ExecutionState, Message},
    };
    for code in [
        ErrorCode::Busy,
        ErrorCode::Unauthorized,
        ErrorCode::RevokedDevice,
    ] {
        for is_write in [true, false] {
            let response = HostAdapter::delivery_refusal(Some("delivery".into()), is_write, code);
            assert_eq!(
                response.execution_state,
                Some(if is_write {
                    ExecutionState::UnknownOutcome
                } else {
                    ExecutionState::Failed
                })
            );
            assert_eq!(response.code, Some(code));
            assert!(matches!(&response.payload, Message::Response { result } if result.is_null()));
            let mut bytes = Vec::new();
            write_frame(&mut bytes, &response).unwrap();
            assert_eq!(read_frame(&mut bytes.as_slice()).unwrap(), Some(response));
        }
    }
}
