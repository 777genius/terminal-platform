use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) struct TestDir {
    path: PathBuf,
}

impl TestDir {
    pub(super) fn new() -> Self {
        let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_nanos(),
            Err(error) => panic!("failed to get test timestamp - {error}"),
        };
        let counter = TEST_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "terminal-platform-xtask-test-{}-{timestamp}-{counter}",
            std::process::id()
        ));
        if let Err(error) = fs::create_dir_all(&path) {
            panic!("failed to create {} - {error}", path.display());
        }
        Self { path }
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn write_file(&self, relative_path: &str, contents: &str) {
        let path = self.path.join(relative_path);
        if let Err(error) = fs::write(&path, contents) {
            panic!("failed to write {} - {error}", path.display());
        }
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
