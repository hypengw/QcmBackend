use sea_orm::entity::prelude::*;
use strum_macros::EnumString;

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, EnumString, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
pub enum CacheType {
    Image = 0,
    Audio = 1,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, EnumString, DeriveActiveEnum)]
#[sea_orm(rs_type = "i32", db_type = "Integer")]
#[strum(ascii_case_insensitive)]
pub enum ImageType {
    Primary = 0,
    Backdrop = 1,
    Banner = 2,
    Thumb = 3,
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
