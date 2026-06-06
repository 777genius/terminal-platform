use std::io;

#[cfg(windows)]
use std::time::Duration;

use interprocess::local_socket::{ListenerOptions, tokio::Listener};
use terminal_protocol::LocalSocketAddress;

pub(super) fn create_listener(address: &LocalSocketAddress) -> io::Result<Listener> {
    #[cfg(windows)]
    {
        let mut last_error = None;
        for attempt in 0..20 {
            match bind_listener(address) {
                Ok(listener) => return Ok(listener),
                Err(error) if attempt < 19 && is_retryable_windows_bind_error(&error) => {
                    last_error = Some(error);
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(error) => return Err(error),
            }
        }

        Err(last_error.unwrap_or_else(|| io::Error::other("listener bind retry failed")))
    }

    #[cfg(not(windows))]
    {
        bind_listener(address)
    }
}

fn bind_listener(address: &LocalSocketAddress) -> io::Result<Listener> {
    ListenerOptions::new().name(address.to_name()?).try_overwrite(true).create_tokio()
}

#[cfg(windows)]
fn is_retryable_windows_bind_error(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::PermissionDenied | io::ErrorKind::AddrInUse | io::ErrorKind::AlreadyExists
    ) || matches!(error.raw_os_error(), Some(5 | 32 | 183))
}
