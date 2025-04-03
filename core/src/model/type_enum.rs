use sea_orm::entity::prelude::*;
use strum_macros::{EnumString, Display};

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, EnumString, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum CacheType {
    Image = 0,
    Audio = 1,
}

#[derive(Debug, Clone, Display, PartialEq, Eq, EnumIter, EnumString, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
#[strum(ascii_case_insensitive)]
pub enum ImageType {
    #[strum(to_string = "Primary")]
    Primary = 0,
    #[strum(to_string = "Backdrop")]
    Backdrop = 1,
    #[strum(to_string = "Banner")]
    Banner = 2,
    #[strum(to_string = "Thumb")]
    Thumb = 3,
    #[strum(to_string = "Logo")]
    Logo = 4,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, EnumString, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
#[strum(ascii_case_insensitive)]
pub enum ItemType {
    Provider = 1,
    Library = 2,

    Album = 51,
    AlbumArtist = 52,
    Artist = 53,
    Mix = 54,
    Radio = 55,

    Song = 101,
    Program = 102,
}
