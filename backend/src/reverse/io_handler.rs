use super::block_store::{block_path, block_path_downloading};
use super::io::IoCmd;
use bytes::{BufMut, Bytes};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// Open file handle cache keyed by cache_key
struct FileCache {
    handles: HashMap<String, File>,
    capacity: usize,
}

impl FileCache {
    fn new(capacity: usize) -> Self {
        Self {
            handles: HashMap::new(),
            capacity,
        }
    }

    fn evict_one(&mut self) {
        if self.handles.len() >= self.capacity {
            if let Some(first_key) = self.handles.keys().next().cloned() {
                self.handles.remove(&first_key);
            }
        }
    }

    /// Open or reuse a file handle with read+write, no truncation.
    fn get_or_open_rw(&mut self, key: &str, path: &Path) -> std::io::Result<&mut File> {
        if !self.handles.contains_key(key) {
            self.evict_one();
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(path)?;
            self.handles.insert(key.to_string(), file);
        }
        Ok(self.handles.get_mut(key).unwrap())
    }

    /// Open or reuse a file handle for reading only.
    fn get_or_open_read(&mut self, key: &str, path: &Path) -> std::io::Result<&mut File> {
        if !self.handles.contains_key(key) {
            self.evict_one();
            let file = File::open(path)?;
            self.handles.insert(key.to_string(), file);
        }
        Ok(self.handles.get_mut(key).unwrap())
    }

    fn remove(&mut self, key: &str) {
        self.handles.remove(key);
    }
}

/// IO thread entry point — pure command executor, no business logic
pub fn io_thread(rx: std::sync::mpsc::Receiver<IoCmd>, cache_dir: PathBuf) {
    let mut file_cache = FileCache::new(128);

    while let Ok(cmd) = rx.recv() {
        match cmd {
            IoCmd::Read {
                key,
                offset,
                len,
                reply,
            } => {
                let result = read_block_sync(&mut file_cache, &cache_dir, &key, offset, len);
                let _ = reply.send(result);
            }
            IoCmd::Write { key, offset, data } => {
                if let Err(e) =
                    write_block_sync(&mut file_cache, &cache_dir, &key, offset, &data)
                {
                    log::error!(target: "io", "write error for {}: {:?}", key, e);
                }
            }
            IoCmd::CreateFile { key } => {
                let path = block_path_downloading(&cache_dir, &key);
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let cache_key = format!("{}.dl", key);
                match OpenOptions::new()
                    .read(true)
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&path)
                {
                    Ok(file) => {
                        file_cache.handles.insert(cache_key, file);
                    }
                    Err(e) => {
                        log::error!(target: "io", "create file error for {}: {:?}", key, e);
                    }
                }
            }
            IoCmd::Rename { key, from, to } => {
                // Close handles before rename
                file_cache.remove(&format!("{}.dl", key));
                file_cache.remove(&key);
                if let Err(e) = fs::rename(&from, &to) {
                    log::error!(target: "io", "rename error {:?} -> {:?}: {:?}", from, to, e);
                }
            }
        }
    }
}

fn read_block_sync(
    file_cache: &mut FileCache,
    cache_dir: &Path,
    key: &str,
    offset: u64,
    len: u64,
) -> Result<Bytes, anyhow::Error> {
    let final_path = block_path(cache_dir, key);
    let dl_path = block_path_downloading(cache_dir, key);

    // For downloading files, use the rw handle (same fd that writes use).
    // For finished files, open read-only.
    let dl_cache_key = format!("{}.dl", key);
    let file = if file_cache.handles.contains_key(&dl_cache_key) {
        // Reuse the rw handle that CreateFile/Write opened — flush first
        let f = file_cache.handles.get_mut(&dl_cache_key).unwrap();
        f.flush()?;
        f
    } else if final_path.exists() {
        file_cache.get_or_open_read(key, &final_path)?
    } else if dl_path.exists() {
        // Downloading file exists but no cached handle — open rw
        file_cache.get_or_open_rw(&dl_cache_key, &dl_path)?
    } else {
        return Err(anyhow::anyhow!("block file not found: {}", key));
    };

    file.seek(SeekFrom::Start(offset))?;

    let read_len = len.min(64 * 1024) as usize;
    let mut buf = vec![0u8; read_len];
    let n = file.read(&mut buf)?;
    buf.truncate(n);

    let mut bytes_buf = bytes::BytesMut::with_capacity(n);
    bytes_buf.put(&buf[..]);
    Ok(bytes_buf.freeze())
}

fn write_block_sync(
    file_cache: &mut FileCache,
    cache_dir: &Path,
    key: &str,
    offset: u64,
    data: &[u8],
) -> std::io::Result<()> {
    let path = block_path_downloading(cache_dir, key);
    let cache_key = format!("{}.dl", key);
    let file = file_cache.get_or_open_rw(&cache_key, &path)?;
    file.seek(SeekFrom::Start(offset))?;
    file.write_all(data)?;
    Ok(())
}
