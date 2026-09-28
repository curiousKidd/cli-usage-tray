use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, Default)]
pub struct Offset {
    pub bytes: u64,
    pub mtime_ms: u128,
}

#[derive(Debug)]
pub struct ReadBatch {
    pub lines: Vec<String>,
    #[cfg_attr(not(test), allow(dead_code))]
    pub next_offset: u64,
}

pub struct OffsetStore {
    dir: PathBuf,
}

impl OffsetStore {
    pub fn in_default_dir() -> io::Result<Self> {
        let home = std::env::var_os("HOME")
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "home directory unavailable"))?;
        Ok(Self::new(PathBuf::from(home).join(".digimon/ingest")))
    }

    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    pub fn read(&self, path: &Path) -> Offset {
        let (offset_path, mtime_path) = self.paths(path);
        Offset {
            bytes: fs::read_to_string(offset_path)
                .ok()
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(0),
            mtime_ms: fs::read_to_string(mtime_path)
                .ok()
                .and_then(|value| value.trim().parse().ok())
                .unwrap_or(0),
        }
    }

    pub fn read_new_lines(&self, path: &Path) -> io::Result<Option<ReadBatch>> {
        let metadata = fs::metadata(path)?;
        let size = metadata.len();
        let mtime_ms = millis(metadata.modified().unwrap_or(UNIX_EPOCH));
        let saved = self.read(path);
        let offset = if size < saved.bytes || mtime_ms < saved.mtime_ms {
            0
        } else {
            saved.bytes
        };
        if size == offset && mtime_ms == saved.mtime_ms {
            return Ok(None);
        }

        let mut file = File::open(path)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let complete = bytes
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map(|index| index + 1)
            .unwrap_or(0);
        let lines = String::from_utf8_lossy(&bytes[..complete])
            .lines()
            .map(str::to_owned)
            .collect();
        let next_offset = offset + complete as u64;
        self.write(path, next_offset, mtime_ms)?;
        Ok(Some(ReadBatch { lines, next_offset }))
    }

    pub fn write(&self, path: &Path, offset: u64, mtime_ms: u128) -> io::Result<()> {
        fs::create_dir_all(&self.dir)?;
        let (offset_path, mtime_path) = self.paths(path);
        fs::write(offset_path, offset.to_string())?;
        fs::write(mtime_path, mtime_ms.to_string())
    }

    fn paths(&self, path: &Path) -> (PathBuf, PathBuf) {
        let mut hasher = DefaultHasher::new();
        path.hash(&mut hasher);
        let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("log");
        let base = format!("{stem}-{:016x}", hasher.finish());
        (self.dir.join(format!("{base}.offset")), self.dir.join(format!("{base}.mtime")))
    }
}

fn millis(time: SystemTime) -> u128 {
    time.duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

#[cfg(test)]
mod tests {
    use super::OffsetStore;
    use std::{fs, time::{SystemTime, UNIX_EPOCH}};

    #[test]
    fn leaves_a_trailing_partial_line_for_the_next_scan() {
        let dir = std::env::temp_dir().join(format!(
            "offset-store-{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        let log = dir.join("session.jsonl");
        fs::create_dir_all(&dir).unwrap();
        fs::write(&log, "one\ntwo\npartial").unwrap();
        let store = OffsetStore::new(dir.join("offsets"));

        let batch = store.read_new_lines(&log).unwrap().unwrap();
        assert_eq!(batch.lines, vec!["one".to_owned(), "two".to_owned()]);
        assert_eq!(batch.next_offset, "one\ntwo\n".len() as u64);
        assert_eq!(store.read(&log).bytes, "one\ntwo\n".len() as u64);

        fs::remove_dir_all(dir).unwrap();
    }
}
