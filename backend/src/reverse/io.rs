use super::connection::RemoteFileInfo;
use super::piece;
use bytes::{BufMut, Bytes, BytesMut};
use qcm_core::model::type_enum::CacheType;
use std::collections::BTreeMap;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum ReadState {
    Reading(u64),
    Paused,
    End,
}
pub struct DownloadFile {
    meta: piece::FileMeta,
    file: std::fs::File,
    pub cache_type: CacheType,
    pub remote_info: RemoteFileInfo,
    pub rc: i64,
}

pub struct Reader {
    file: std::fs::File,
    key: String,
    piece: piece::Piece,
    state: ReadState,
}

pub struct Waiter {
    key: String,
    start: u64,
}

type Writers = BTreeMap<String, DownloadFile>;
type Waiters = BTreeMap<i64, Waiter>;
type Readers = BTreeMap<i64, Reader>;

pub struct IoContext {
    writers: Writers,
    readers: Readers,
    waiters: Waiters,
    cache_dir: PathBuf,
}

impl IoContext {
    pub fn new(cache_dir: &Path) -> Self {
        Self {
            writers: Writers::new(),
            readers: Readers::new(),
            waiters: Waiters::new(),
            cache_dir: cache_dir.to_path_buf(),
        }
    }

    pub fn create_cache_file(&self, key: &str) -> std::io::Result<(std::fs::File, PathBuf)> {
        let dir = self.cache_dir.join(key.get(0..2).unwrap_or("00"));
        let file = dir.join(key).with_extension("downloading");
        let _ = std::fs::create_dir_all(&dir)?;
        log::debug!(target: "reverse", "new file: {}", file.to_string_lossy());
        std::fs::File::create(&file).map(|f| (f, file))
    }

    pub fn get_cache_file(
        &self,
        key: &str,
        cursor: u64,
    ) -> std::io::Result<(std::fs::File, u64, PathBuf)> {
        let dir = self.cache_dir.join(key.get(0..2).unwrap_or("00"));
        let file = dir.join(key);
        std::fs::File::open(&file).and_then(|mut f| {
            f.seek(std::io::SeekFrom::End(0))?;
            let len = f.stream_position()?;
            f.seek(std::io::SeekFrom::Start(cursor))?;
            Ok((f, len, file))
        })
    }

    fn end_file(f: &DownloadFile, key: &str, readers: &mut Readers) {
        let path = f.meta.path.clone();

        log::debug!(target: "reverse", "finish file: {}", key);
        let mut tmp = BTreeMap::<i64, (piece::Piece, ReadState)>::new();
        for (id, v) in readers.iter() {
            if v.key == key {
                tmp.insert(*id, (v.piece.clone(), v.state.clone()));
            }
        }
        for (id, _) in tmp.iter() {
            readers.remove(id);
        }
        let old = path;
        let path = old.with_file_name(old.file_stem().and_then(|s| s.to_str()).unwrap_or(key));

        if let Err(err) = std::fs::rename(&old, &path) {
            log::error!(target: "reverse", "{:?}", err);
        }

        for (id, (piece, state)) in tmp {
            match std::fs::File::open(&path).and_then(|mut f| {
                f.seek(std::io::SeekFrom::Start(
                    piece.offset
                        + match state {
                            ReadState::Reading(o) => o,
                            _ => 0,
                        },
                ))?;
                Ok(f)
            }) {
                Ok(file) => {
                    readers.insert(
                        id,
                        Reader {
                            file,
                            key: key.to_string(),
                            piece,
                            state,
                        },
                    );
                }
                _ => continue,
            }
        }
    }

    pub fn do_write(
        &mut self,
        key: &str,
        bytes: &Bytes,
        offset: u64,
        on_notify: impl Fn(&str, i64, u64),
        on_finish: impl Fn(&str, &DownloadFile),
    ) -> std::io::Result<()> {
        let f = match self.writers.get_mut(key) {
            Some(f) => f,
            None => {
                return Ok(());
            }
        };
        let len = bytes.len() as u64;
        match f.meta.combine(piece::Piece {
            offset,
            length: len,
        }) {
            true => {
                f.file.seek(std::io::SeekFrom::Start(offset))?;
                f.file.write(&bytes)?;
            }
            false => {}
        }

        {
            let mut ids = Vec::new();
            for (id, v) in self.waiters.iter() {
                if v.key == key && v.start >= offset && v.start < offset + len {
                    ids.push(*id);
                    on_notify(&v.key, *id, v.start);
                }
            }
            for id in ids {
                self.waiters.remove(&id);
            }
        }

        if f.meta.is_end() {
            Self::end_file(f, key, &mut self.readers);
            on_finish(key, f);
            self.writers.remove(key);
        }

        Ok(())
    }

    pub fn end(&mut self, id: i64) {
        self.readers.remove(&id);
        self.waiters.remove(&id);
    }

    pub fn end_writer(&mut self, key: &str) {
        if let Some(w) = self.writers.get_mut(key) {
            w.rc -= 1;
            if w.rc == 0 {
                self.writers.remove(key);
            }
        }
    }

    pub fn get_piece(&mut self, key: &str, id: i64, cursor: u64) -> Option<piece::Piece> {
        match self.writers.get_mut(key) {
            Some(f) => match f.meta.piece_of(cursor) {
                Some(p) => {
                    let _ = f.file.flush();
                    match self.readers.get_mut(&id) {
                        Some(reader) => match reader.file.seek(std::io::SeekFrom::Start(cursor)) {
                            Err(e) => {
                                log::error!("{:?}", e);
                                None
                            }
                            _ => {
                                reader.piece = p.clone();
                                reader.state = ReadState::Reading(0);
                                Some(p)
                            }
                        },
                        None => match std::fs::File::open(&f.meta.path) {
                            Ok(mut file) => match file.seek(std::io::SeekFrom::Start(cursor)) {
                                Err(e) => {
                                    log::error!("{:?}", e);
                                    None
                                }
                                _ => {
                                    self.readers.insert(
                                        id,
                                        Reader {
                                            file: file,
                                            key: key.to_string(),
                                            piece: p.clone(),
                                            state: ReadState::Reading(0),
                                        },
                                    );
                                    Some(p)
                                }
                            },
                            Err(e) => {
                                log::error!("{:?}", e);
                                None
                            }
                        },
                    }
                }
                None => None,
            },
            None => match self.get_cache_file(&key, cursor) {
                Ok((file, len, _)) => {
                    let p = piece::Piece {
                        offset: cursor,
                        length: len - cursor,
                    };
                    self.readers.insert(
                        id,
                        Reader {
                            file: file,
                            key: key.to_string(),
                            piece: p.clone(),
                            state: ReadState::Reading(0),
                        },
                    );
                    Some(p)
                }
                Err(_) => None,
            },
        }
    }

    pub fn add_waiter(&mut self, id: i64, key: &str, start: u64) {
        self.waiters.insert(
            id,
            Waiter {
                key: key.to_string(),
                start,
            },
        );
    }

    pub fn set_reader_state(&mut self, id: i64, state: ReadState) -> bool {
        if let Some(reader) = self.readers.get_mut(&id) {
            reader.state = state;
            return true;
        }
        return false;
    }

    pub fn new_write(
        &mut self,
        key: &str,
        len: u64,
        cache_type: CacheType,
        remote_info: RemoteFileInfo,
    ) -> bool {
        match self.writers.get_mut(key) {
            None => match self.create_cache_file(key) {
                Ok((file, path)) => {
                    self.writers.insert(
                        key.to_string(),
                        DownloadFile {
                            meta: piece::FileMeta {
                                path,
                                len,
                                pieces: BTreeMap::new(),
                            },
                            cache_type,
                            file,
                            remote_info,
                            rc: 1,
                        },
                    );
                    return true;
                }
                Err(e) => {
                    log::error!("{:?}", e);
                    return false;
                    // continue;
                }
            },
            Some(w) => {
                w.rc += 1;
                return true;
            }
        }
    }

    pub fn reading_count(&self) -> u64 {
        let mut out = 0;
        for (_, v) in self.readers.iter() {
            if let ReadState::Reading(_) = v.state {
                out += 1
            }
        }
        return out;
    }

    pub fn do_read(&mut self, on_ok: impl Fn(i64, ReadState, Bytes), on_fail: impl Fn(i64)) {
        // 64K
        let mut buf = [0; 64 * 1024];
        self.readers
            .iter_mut()
            .filter(|(_, v)| match v.state {
                ReadState::Reading(_) => true,
                _ => false,
            })
            .for_each(|(id, v)| {
                let mut readed = match v.state {
                    ReadState::Reading(o) => o,
                    _ => 0,
                };
                let len = std::cmp::min((v.piece.length - readed) as usize, 64 * 1024);

                match v.file.read(&mut buf[0..len]) {
                    Ok(size) => {
                        let mut bytes_buf = bytes::BytesMut::new();
                        bytes_buf.put(&buf[0..size]);
                        if size == 0 {
                            log::error!("readed zero, {}/{}", readed, v.piece.length);
                            on_fail(*id);
                        } else {
                            readed += size as u64;
                            if readed == v.piece.length {
                                v.state = ReadState::End;
                            } else {
                                if readed >= 64 * 1024 {
                                    v.piece.length -= readed;
                                    v.state = ReadState::Paused;
                                } else {
                                    v.state = ReadState::Reading(readed);
                                }
                            }
                            on_ok(*id, v.state.clone(), bytes_buf.freeze());
                        }
                    }
                    Err(e) => {
                        log::error!("{:?}", e);
                        on_fail(*id);
                    }
                }
            });
    }
}
