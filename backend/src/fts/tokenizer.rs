use unicode_normalization::UnicodeNormalization;

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Alphabetic(String, usize),
    Numeric(String, usize),
    NGram(String, usize),
}

#[derive(Debug, PartialEq)]
enum CharType {
    Alphabetic,
    Numeric,
    Other,
    Separator,
}

#[derive(Debug)]
pub struct FtsTokenizer {}

impl FtsTokenizer {
    pub fn new() -> Self {
        FtsTokenizer {}
    }

    fn get_char_type(c: char) -> CharType {
        if c.is_ascii_alphabetic() {
            CharType::Alphabetic
        } else if c.is_ascii_digit() {
            CharType::Numeric
        } else if (c as u32) < 0x80 {
            CharType::Separator
        } else {
            CharType::Other
        }
    }

    fn process_token(token: &str, char_type: CharType, start_pos: usize) -> Vec<Token> {
        if token.is_empty() {
            return vec![];
        }

        match char_type {
            CharType::Numeric => vec![Token::Numeric(token.to_string(), start_pos)],
            CharType::Alphabetic => vec![Token::Alphabetic(token.to_lowercase(), start_pos)],
            CharType::Other => {
                let chars: Vec<char> = token.chars().collect();
                if chars.len() < 2 {
                    vec![Token::NGram(token.to_string(), start_pos)]
                } else {
                    let mut pos = start_pos;
                    chars
                        .windows(2)
                        .map(|window| {
                            let token = Token::NGram(window.iter().collect::<String>(), pos);
                            pos += 1;
                            token
                        })
                        .collect()
                }
            }
            CharType::Separator => vec![],
        }
    }

    pub fn tokenize(&self, input: &str) -> Vec<Token> {
        let normalized = input.nfkc().collect::<String>();
        let mut result = Vec::new();
        let mut current_token = String::new();
        let mut current_type = CharType::Separator;
        let mut current_start = 0;
        let mut pos = 0;

        for c in normalized.chars() {
            let char_type = Self::get_char_type(c);

            if char_type != current_type && !current_token.is_empty() {
                result.extend(Self::process_token(
                    &current_token,
                    current_type,
                    current_start,
                ));
                current_token.clear();
                current_start = pos;
            }

            if char_type != CharType::Separator {
                if current_token.is_empty() {
                    current_start = pos;
                }
                current_token.push(c);
            }
            current_type = char_type;
            pos += 1;
        }

        if !current_token.is_empty() {
            result.extend(Self::process_token(
                &current_token,
                current_type,
                current_start,
            ));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer() {
        let tokenizer = FtsTokenizer::new();

        // Test alphabetic (lowercase)
        assert_eq!(
            tokenizer.tokenize("Hello"),
            vec![Token::Alphabetic("hello".to_string(), 0)]
        );

        // Test numeric
        assert_eq!(
            tokenizer.tokenize("12345"),
            vec![Token::Numeric("12345".to_string(), 0)]
        );

        // Test mixed with separators
        assert_eq!(
            tokenizer.tokenize("Hello-World"),
            vec![
                Token::Alphabetic("hello".to_string(), 0),
                Token::Alphabetic("world".to_string(), 6)
            ]
        );

        // Test Chinese characters (ngram)
        assert_eq!(
            tokenizer.tokenize("中国"),
            vec![Token::NGram("中国".to_string(), 0)]
        );
        assert_eq!(
            tokenizer.tokenize("中国人"),
            vec![
                Token::NGram("中国".to_string(), 0),
                Token::NGram("国人".to_string(), 1)
            ]
        );

        // Test mixed content
        assert_eq!(
            tokenizer.tokenize("Hello世界123"),
            vec![
                Token::Alphabetic("hello".to_string(), 0),
                Token::NGram("世界".to_string(), 5),
                Token::Numeric("123".to_string(), 7)
            ]
        );

        // Test mixed without separators
        assert_eq!(
            tokenizer.tokenize("abc123世界def"),
            vec![
                Token::Alphabetic("abc".to_string(), 0),
                Token::Numeric("123".to_string(), 3),
                Token::NGram("世界".to_string(), 6),
                Token::Alphabetic("def".to_string(), 8)
            ]
        );

        // Test mixed with Chinese
        assert_eq!(
            tokenizer.tokenize("我abc123"),
            vec![
                Token::NGram("我".to_string(), 0),
                Token::Alphabetic("abc".to_string(), 1),
                Token::Numeric("123".to_string(), 4)
            ]
        );

        // Test mixed numbers and letters
        assert_eq!(
            tokenizer.tokenize("abc123def456"),
            vec![
                Token::Alphabetic("abc".to_string(), 0),
                Token::Numeric("123".to_string(), 3),
                Token::Alphabetic("def".to_string(), 6),
                Token::Numeric("456".to_string(), 9)
            ]
        );
    }
}
