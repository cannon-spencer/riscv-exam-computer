//! Atomic JSON persistence: write a temp file, fsync, rename over the old one.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::fleet::Snapshot;

pub struct Store {
    path: PathBuf,
}

impl Store {
    pub fn new(dir: &Path) -> std::io::Result<Self> {
        fs::create_dir_all(dir)?;
        Ok(Store { path: dir.join("state.json") })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Loads the snapshot. A missing file is an empty state; a corrupt one
    /// is moved aside (never deleted) and reported.
    pub fn load(&self) -> (Snapshot, Option<String>) {
        let bytes = match fs::read(&self.path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return (Snapshot::default(), None),
            Err(e) => return (Snapshot::default(), Some(format!("could not read {}: {e}", self.path.display()))),
        };
        match serde_json::from_slice(&bytes) {
            Ok(s) => (s, None),
            Err(e) => {
                let aside = self.path.with_extension(format!("corrupt-{}.json", std::process::id()));
                let _ = fs::rename(&self.path, &aside);
                (Snapshot::default(), Some(format!("{} was unreadable ({e}); moved to {}", self.path.display(), aside.display())))
            }
        }
    }

    pub fn save(&self, snap: &Snapshot) -> std::io::Result<()> {
        let tmp = self.path.with_extension("json.tmp");
        {
            let mut f = fs::File::create(&tmp)?;
            serde_json::to_writer(&mut f, snap)?;
            f.write_all(b"\n")?;
            f.sync_all()?;
        }
        fs::rename(&tmp, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tempdir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("exam-store-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn save_load_roundtrip_and_corrupt_file_is_kept() {
        let dir = tempdir("rt");
        let store = Store::new(&dir).unwrap();
        assert_eq!(store.load().0, Snapshot::default());
        let snap = Snapshot { version: 1, next_event_id: 9, ..Default::default() };
        store.save(&snap).unwrap();
        assert_eq!(store.load().0, snap);

        fs::write(store.path(), b"{not json").unwrap();
        let (s, warn) = store.load();
        assert_eq!(s, Snapshot::default());
        assert!(warn.unwrap().contains("unreadable"));
        assert!(fs::read_dir(&dir).unwrap().any(|e| e.unwrap().file_name().to_string_lossy().contains("corrupt")));
        let _ = fs::remove_dir_all(&dir);
    }
}
