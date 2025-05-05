use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone)]
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
        self.pieces
            .range(..=start)
            .next_back()
            .filter(|(_, p)| p.offset + p.length > start)
            .map(|(_, p)| p.clone())
    }

    pub fn combine(&mut self, p: Piece) {
        let mut merged = p.clone();
        let mut to_remove = Vec::new();

        for (offset, existing) in self.pieces.range(..) {
            if existing.offset + existing.length == p.offset {
                merged.offset = existing.offset;
                merged.length += existing.length;
                to_remove.push(*offset);
            } else if p.offset + p.length == existing.offset {
                merged.length += existing.length;
                to_remove.push(*offset);
            }
        }

        for offset in to_remove {
            self.pieces.remove(&offset);
        }
        self.pieces.insert(merged.offset, merged);
    }

    pub fn is_end(&self) -> bool {
        return self.pieces.len() == 1
            && self
                .pieces
                .get(&0)
                .map(|v| v.offset == 0 && v.length == self.len)
                .unwrap_or(false);
    }
}
