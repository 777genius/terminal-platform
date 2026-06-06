mod bootstrap_smoke {
    mod prelude;
    mod support;

    mod daemon_native;
    mod fullscreen;
    mod native_layout;
    #[cfg(any(unix, windows))]
    mod saved_sessions;
    #[cfg(unix)]
    mod tmux;
    mod zellij;
}
