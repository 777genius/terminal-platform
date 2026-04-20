# Fuzz Targets

`fuzz/` stays outside the main workspace on purpose.

Current first-class targets cover:

- protocol frame and envelope decoding
- `tmux` layout parsing
- `Zellij` probe and JSON parser seams
- screen delta apply/merge hot paths

Local usage:

```bash
cargo fuzz run protocol_frames
cargo fuzz run tmux_layout
cargo fuzz run zellij_surface
cargo fuzz run screen_delta
```

CI uses a short baseline run to ensure the targets stay buildable and executable.
