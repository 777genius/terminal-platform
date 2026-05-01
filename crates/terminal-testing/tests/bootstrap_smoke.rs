mod bootstrap_smoke {
    mod prelude;
    mod support;

    mod daemon_native;
    mod fullscreen;
    mod native_layout;
    #[cfg(unix)]
    mod saved_sessions;
    #[cfg(unix)]
    mod tmux;
    mod zellij;
}
