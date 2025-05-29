use serde::{Deserialize, Serialize};
mod lrc;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SubtitleItem {
    pub start: Option<i64>,
    pub end: Option<i64>,
    pub text: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Subtitle {
    pub items: Vec<SubtitleItem>,
}

impl Subtitle {
    pub fn from_lrc(lrc: &str) -> Option<Self> {
        None
        // lrc::parse(lrc).ok().map(|lrc| {
        //     let items = lrc
        //         .into_iter()
        //         .map(|item| SubtitleItem {
        //             start: item.start,
        //             end: item.end,
        //             text: item.text.map(|text| text.to_string()),
        //         })
        //         .collect();
        //     Subtitle { items }
        // })
    }
}
