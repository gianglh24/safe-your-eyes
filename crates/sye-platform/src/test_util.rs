use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEST_DIR_SEQ: AtomicU64 = AtomicU64::new(0);

pub(crate) struct TestTempDir {
    path: PathBuf,
}

impl TestTempDir {
    pub(crate) fn new(prefix: &str) -> io::Result<Self> {
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir
            .parent()
            .and_then(Path::parent)
            .unwrap_or(manifest_dir);
        let base = repo_root.join(".tmp");
        fs::create_dir_all(&base)?;

        let seq = TEST_DIR_SEQ.fetch_add(1, Ordering::Relaxed);
        let dir = base.join(format!("t-{prefix}-{}-{seq}", std::process::id()));
        fs::create_dir_all(&dir)?;
        Ok(Self { path: dir })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
