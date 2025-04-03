pub mod values;
pub use const_chunks::IteratorConstChunks;

pub fn columns_contains<Col>(list: &[Col], c: &Col) -> bool {
    use std::mem::discriminant;
    for l in list {
        if discriminant(l) == discriminant(c) {
            return true;
        }
    }
    return false;
}
