use terminal_protocol::ProtocolError;

pub(super) fn is_subscription_close_disconnect(error: &ProtocolError) -> bool {
    if !matches!(error.code.as_str(), "send_failed" | "receive_failed") {
        return false;
    }

    let message = error.message.to_ascii_lowercase();
    message.contains("broken pipe")
        || message.contains("connection reset")
        || message.contains("not connected")
        || message.contains("unexpected eof")
        || message.contains("the pipe is being closed")
        || message.contains("os error 109")
        || message.contains("os error 232")
        || message.contains("os error 233")
        || message.contains("the handle is invalid")
}

#[cfg(test)]
mod tests {
    use terminal_protocol::ProtocolError;

    use super::is_subscription_close_disconnect;

    #[test]
    fn subscription_close_disconnect_recognizes_windows_pipe_error_codes() {
        for raw_os_message in [
            "localized pipe disconnect (os error 109)",
            "localized pipe closing (os error 232)",
            "localized pipe peer missing (os error 233)",
        ] {
            let error = ProtocolError::new("send_failed", raw_os_message);
            assert!(is_subscription_close_disconnect(&error));
        }
    }
}
