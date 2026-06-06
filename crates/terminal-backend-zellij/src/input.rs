use crate::action::ZellijAction;

pub(crate) fn flush_zellij_literal(
    pane_ref: &str,
    literal: &mut String,
    actions: &mut Vec<ZellijAction>,
) {
    if literal.is_empty() {
        return;
    }

    actions
        .push(ZellijAction::WriteChars { pane_ref: pane_ref.to_string(), chars: literal.clone() });
    literal.clear();
}

pub(crate) fn zellij_named_key_sequence(input: &str) -> Option<(&'static str, &'static str)> {
    [
        ("\u{1b}[3~", "Delete"),
        ("\u{1b}[5~", "PageUp"),
        ("\u{1b}[6~", "PageDown"),
        ("\u{1b}[A", "Up"),
        ("\u{1b}[B", "Down"),
        ("\u{1b}[C", "Right"),
        ("\u{1b}[D", "Left"),
        ("\u{1b}[H", "Home"),
        ("\u{1b}[F", "End"),
        ("\u{1b}", "Esc"),
    ]
    .into_iter()
    .find(|(sequence, _)| input.starts_with(sequence))
}

pub(crate) fn zellij_control_key(ch: char) -> Option<&'static str> {
    match ch {
        '\u{0001}' => Some("Ctrl a"),
        '\u{0003}' => Some("Ctrl c"),
        '\u{0004}' => Some("Ctrl d"),
        '\u{0005}' => Some("Ctrl e"),
        '\u{000b}' => Some("Ctrl k"),
        '\u{000c}' => Some("Ctrl l"),
        '\u{0015}' => Some("Ctrl u"),
        '\u{0017}' => Some("Ctrl w"),
        '\u{007f}' => Some("Backspace"),
        _ => None,
    }
}
