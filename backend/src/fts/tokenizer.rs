

#[derive(Debug)]
pub struct FtsTokenizer {
    text: String,
}

impl FtsTokenizer {
    pub fn new() -> Self {
        FtsTokenizer {
            text: String::new(),
        }
    }

    pub fn tokenize(&self, input: &str) -> Vec<String> {
        input.split_whitespace().map(String::from).collect()
    }
}