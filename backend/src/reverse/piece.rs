use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug,Clone)]
pub struct Piece {
    pub offset: u64,
    pub length: u64,
}

pub struct FileMeta {
    pub path: PathBuf,
    pub len: u64,
    pub pieces: BTreeMap<u64, Piece>,
}

impl FileMeta {
    pub fn longest_piece(&self, start: u64) -> Option<Piece> {
        None
    }

    pub fn combine(&self, p: Piece) {}

    pub fn is_end(&self) -> bool {
        return self.pieces.len() == 1
            && self
                .pieces
                .get(&0)
                .map(|v| v.offset == 0 && v.length == self.len)
                .unwrap_or(false);
    }
}
