use super::error;
use nom::{
    branch::alt,
    bytes::complete::{is_a, is_not, tag, take_until, take_while},
    character::complete::{
        alpha1, anychar, char, digit1, line_ending, multispace0, none_of, not_line_ending, space0,
    },
    combinator::opt,
    error::ParseError,
    multi::many1,
    sequence::{delimited, preceded, separated_pair},
    Parser,
};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Debug, PartialEq, Eq)]
pub enum LrcMetadata<'a> {
    /// Artist of the song
    Artist(&'a str),
    /// Album this song belongs to
    Album(&'a str),
    /// Title of the song
    Title(&'a str),
    /// Lyricist wrote this songtext
    Lyricist(&'a str),
    /// Author of this LRC
    Author(&'a str),
    /// Length of the song
    Length(&'a str),
    /// Offset in milliseconds
    Offset(i64),
    /// The player or editor that created the LRC file
    Application(&'a str),
    /// version of the app above
    AppVersion(&'a str),
    /// Comments
    Comment(&'a str),
    Unkown(&'a str),
}

#[derive(Debug, PartialEq, Eq)]
pub enum LrcTag<'a> {
    Id(LrcMetadata<'a>),
    /// Lyric text and timestamp in milliseconds without offset
    Time(&'a str, Vec<i64>),
}

#[derive(Debug, PartialEq, Eq)]
enum Content<'a> {
    Tag(LrcTag<'a>),
    Unkown(&'a str),
}

type IResult<I, O, E = error::Error<I>> = Result<(I, O), nom::Err<E>>;

fn parse_timestamp(input: &str) -> IResult<&str, i64> {
    let num_str = |input| -> IResult<&str, &str> { delimited(space0, digit1, space0).parse(input) };
    let (remaining, (n1, (n2, n3))) = separated_pair(
        num_str,
        tag(":"),
        separated_pair(num_str, is_a(":."), num_str),
    )
    .parse(input)?;

    let min = Decimal::from_str_exact(n1)
        .map_err(|_| (error::Error::nom(n1, error::ErrorKind::InvalidTimestamp)))?;

    let sec = Decimal::from_str_exact(n2)
        .map_err(|_| (error::Error::nom(n2, error::ErrorKind::InvalidTimestamp)))?;

    let sec_decimal = {
        let mut num = Decimal::from_str_exact(n3)
            .map_err(|_| (error::Error::nom(n3, error::ErrorKind::InvalidTimestamp)))?;
        num.set_scale(n3.len() as u32)
            .map_err(|_| (error::Error::nom(n2, error::ErrorKind::InvalidTimestamp)))?;
        num
    };

    let timestamp = (min * dec!(60) + sec + sec_decimal) * dec!(1000);

    let timestamp_i64 = timestamp
        .trunc()
        .to_i64()
        .ok_or_else(|| error::Error::nom(remaining, error::ErrorKind::InvalidTimestamp))?;

    Ok((remaining, timestamp_i64))
}

fn parse_time_tag_timestamp(input: &str) -> IResult<&str, i64> {
    delimited(tag("["), parse_timestamp, tag("]")).parse(input)
}

fn parse_id_tag(input: &str) -> IResult<&str, LrcTag> {
    let (remaining, (id, value_)) = delimited(
        tag("["),
        separated_pair(
            delimited(space0, alpha1, space0),
            tag(":"),
            delimited(space0, is_not("]"), space0),
        ),
        tag("]"),
    )
    .parse(input)?;

    let value = value_.trim_end();
    let metadata = match id {
        "ar" => LrcMetadata::Artist(value),
        "al" => LrcMetadata::Album(value),
        "ti" => LrcMetadata::Title(value),
        "au" => LrcMetadata::Lyricist(value),
        "length" => LrcMetadata::Length(value),
        "by" => LrcMetadata::Author(value),
        "offset" => {
            let offset = value
                .parse::<i64>()
                .map_err(|_| error::Error::nom(value, error::ErrorKind::InvalidOffset))?;
            LrcMetadata::Offset(offset)
        }
        "re" => LrcMetadata::Application(value),
        "ve" => LrcMetadata::AppVersion(value),
        "#" => LrcMetadata::Comment(value),
        _ => LrcMetadata::Unkown(value),
    };
    Ok((remaining, LrcTag::Id(metadata)))
}

fn parse_time_tag(input: &str) -> IResult<&str, LrcTag> {
    let (remaining, (timestamps, _, text)) =
        (many1(parse_time_tag_timestamp), space0, not_line_ending).parse(input)?;

    Ok((remaining, LrcTag::Time(text.trim_end(), timestamps)))
}

pub fn parse_tag(input: &str) -> IResult<&str, LrcTag> {
    preceded(space0, alt((parse_time_tag, parse_id_tag))).parse(input)
}

fn parse_content(input: &str) -> IResult<&str, Content> {
    let parse_tag = |input| parse_tag(input).map(|(r, t)| (r, Content::Tag(t)));
    let parse_unknown = |input| not_line_ending(input).map(|(r, t)| (r, Content::Unkown(t)));

    let (remaining, (content, _)) =
        (alt((parse_tag, parse_unknown)), opt(line_ending)).parse(input)?;
    Ok((remaining, content))
}

pub fn parse(input: &str) -> Result<Vec<LrcTag>, error::Error<&str>> {
    let mut result = Vec::new();
    let mut remaining = input;

    while !remaining.is_empty() {
        match parse_content(remaining) {
            Ok((rest, content)) => {
                match content {
                    Content::Tag(tag) => {
                        result.push(tag);
                    }
                    Content::Unkown(_unk) => {}
                }
                remaining = rest;
            }
            Err(nom::Err::Incomplete(_)) => {
                return Err(error::Error::new(remaining, error::ErrorKind::Incomplete))
            }
            Err(nom::Err::Error(e)) => return Err(e),
            Err(nom::Err::Failure(e)) => return Err(e),
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_timestamp() {
        {
            let (_, ts) = parse_timestamp("01:23.45").unwrap();
            assert_eq!(ts, 83450);
        }
        {
            let (_, ts) = parse_time_tag_timestamp("[00:23:45]").unwrap();
            assert_eq!(ts, 23450);
        }
    }

    #[test]
    fn test_parse_metadata() {
        let input = "[ti:Title]";
        let (_, item) = parse_tag(input).unwrap();
        match item {
            LrcTag::Id(LrcMetadata::Title(title)) => assert_eq!(title, "Title"),
            _ => panic!("Expected Title metadata"),
        }
    }

    #[test]
    fn test_parse_lyric() {
        let input = "[00:01.23]Hello World";
        let (_, item) = parse_time_tag(input).unwrap();
        match item {
            LrcTag::Time(text, timestamps) => {
                assert_eq!(text, "Hello World");
                assert_eq!(timestamps, vec![1230]);
            }
            _ => panic!("Expected Lyric"),
        }
    }

    #[test]
    fn test_parse_multiple_timestamps() {
        let input = "[00:01.00][00:02.00]Multiple timestamps";
        let (_, item) = parse_time_tag(input).unwrap();
        match item {
            LrcTag::Time(text, timestamps) => {
                assert_eq!(text, "Multiple timestamps");
                assert_eq!(timestamps, vec![1000, 2000]);
            }
            _ => panic!("Expected Lyric"),
        }
    }

    #[test]
    fn test_parse_complete_lrc() {
        let input = "
[ti:Song Title]
[ar:Artist]
[00:01.00]First line
[00:02.00]Second line
[00:03.00][00:04.00]Multiple timestamps";

        let items = parse(input).unwrap();
        assert_eq!(items.len(), 5);

        match &items[0] {
            LrcTag::Id(LrcMetadata::Title(title)) => assert_eq!(*title, "Song Title"),
            _ => panic!("Expected Title metadata"),
        }

        match &items[1] {
            LrcTag::Id(LrcMetadata::Artist(artist)) => assert_eq!(*artist, "Artist"),
            _ => panic!("Expected Artist metadata"),
        }

        match &items[2] {
            LrcTag::Time(text, timestamps) => {
                assert_eq!(*text, "First line");
                assert_eq!(*timestamps, vec![1000]);
            }
            _ => panic!("Expected Lyric"),
        }
    }
}
