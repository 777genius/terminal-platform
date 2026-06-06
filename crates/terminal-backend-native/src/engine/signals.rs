use tokio::sync::watch;

pub(super) fn bump_watch(sender: &watch::Sender<u64>) {
    let next = sender.borrow().wrapping_add(1);
    let _ = sender.send(next);
}
