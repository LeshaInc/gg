use std::fmt::{self, Debug};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use gg_util::eyre::{Result, WrapErr};
use gg_util::parking_lot::Mutex;
use notify::{DebouncedEvent, RecommendedWatcher, RecursiveMode, Watcher};
use tracing::error;

pub trait Source: Send + Sync + Debug + 'static {
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>>;

    fn read_string(&self, path: &Path) -> Result<String> {
        let bytes = self.read_bytes(path)?;
        String::from_utf8(bytes).wrap_err("invalid utf-8")
    }

    fn start_watching(&self, callback: Box<dyn Fn(&Path) + Send + Sync + 'static>) {
        let _ = callback;
    }
}

pub struct DirSource {
    root: PathBuf,
    watch_data: Mutex<Option<(RecommendedWatcher, JoinHandle<()>)>>,
}

impl Debug for DirSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DirSource")
            .field("root", &self.root)
            .finish_non_exhaustive()
    }
}

impl DirSource {
    pub fn new(root: impl AsRef<Path>) -> Result<DirSource> {
        Ok(DirSource {
            root: root.as_ref().canonicalize()?,
            watch_data: Mutex::new(None),
        })
    }

    fn start_watching_inner(
        &self,
        callback: Box<dyn Fn(&Path) + Send + Sync + 'static>,
    ) -> Result<()> {
        let (tx, rx) = mpsc::channel();
        let delay = Duration::from_millis(50);
        let mut watcher = notify::watcher(tx, delay)?;
        watcher.watch(&self.root, RecursiveMode::Recursive)?;

        let root = self.root.clone();
        let handle = thread::Builder::new()
            .name("asset-watcher".into())
            .spawn(move || {
                for event in rx.iter() {
                    if let DebouncedEvent::Create(path) | DebouncedEvent::Write(path) = event {
                        if let Ok(suffix) = path.strip_prefix(&root) {
                            callback(suffix);
                        }
                    }
                }
            })?;

        *self.watch_data.lock() = Some((watcher, handle));

        Ok(())
    }
}

impl Source for DirSource {
    fn read_bytes(&self, path: &Path) -> Result<Vec<u8>> {
        let file_path = self.root.join(&path);
        let mut file = File::open(&file_path)
            .wrap_err_with(|| format!("cannot open {}", file_path.display()))?;

        let meta = file.metadata().ok();
        let capacity = meta
            .and_then(|meta| usize::try_from(meta.len()).ok())
            .unwrap_or(0);

        let mut buf = Vec::with_capacity(capacity);
        file.read_to_end(&mut buf)
            .wrap_err_with(|| format!("cannot read {}", file_path.display()))?;

        Ok(buf)
    }

    fn start_watching(&self, callback: Box<dyn Fn(&Path) + Send + Sync + 'static>) {
        if let Err(error) = self.start_watching_inner(callback) {
            error!(?error, "file watching error");
        };
    }
}
