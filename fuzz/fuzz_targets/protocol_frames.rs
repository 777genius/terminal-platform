#![no_main]

use libfuzzer_sys::fuzz_target;
use terminal_protocol::{
    RequestEnvelope, ResponseEnvelope, SubscriptionEnvelope, TransportResponse,
    decode_json_frame, encode_json_frame,
};

fuzz_target!(|data: &[u8]| {
    if let Ok(envelope) = decode_json_frame::<RequestEnvelope>(data) {
        let _ = encode_json_frame(&envelope);
    }

    if let Ok(envelope) = decode_json_frame::<ResponseEnvelope>(data) {
        let _ = encode_json_frame(&envelope);
    }

    if let Ok(envelope) = decode_json_frame::<SubscriptionEnvelope>(data) {
        let _ = encode_json_frame(&envelope);
    }

    if let Ok(response) = decode_json_frame::<TransportResponse>(data) {
        let _ = encode_json_frame(&response);
    }
});
