use super::util::{default_json_arr, default_true, epoch};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};
use super::type_enum::ItemType;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "song")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub library_id: i64,
    pub name: String,
    #[serde(default)]
    pub sort_name: Option<String>,
    pub native_id: String,
    #[sea_orm(nullable)]
    pub album_id: Option<i64>,
    pub track_number: i32,
    pub disc_number: i32,
    pub duration: i64,
    #[serde(default = "default_true")]
    pub can_play: bool,
    #[serde(default)]
    pub popularity: f64,
    #[serde(default = "epoch")]
    pub publish_time: DateTimeUtc,
    #[serde(default = "default_json_arr")]
    pub tags: Json,
    #[serde(default = "chrono::Utc::now")]
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub edit_time: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::library::Entity",
        from = "Column::LibraryId",
        to = "super::library::Column::LibraryId"
    )]
    Library,
    #[sea_orm(
        belongs_to = "super::album::Entity",
        from = "Column::AlbumId",
        to = "super::album::Column::Id"
    )]
    Album,
    #[sea_orm(
        has_many = "super::image::Entity",
        on_condition = r#"Expr::col(super::image::Column::ItemType).eq(ItemType::Song)"#
    )]
    Image,
    #[sea_orm(
        has_many = "super::favorite::Entity",
        on_condition = r#"Expr::col(super::favorite::Column::ItemType).eq(ItemType::Song)"#
    )]
    Favorite,
}

impl Related<super::library::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Library.def()
    }
}

impl Related<super::album::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Album.def()
    }
}
impl Related<super::image::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Image.def()
    }
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        super::rel_song_artist::Relation::Artist.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::rel_song_artist::Relation::Song.def().rev())
    }
}

impl Related<super::mix::Entity> for Entity {
    fn to() -> RelationDef {
        super::rel_mix_song::Relation::Mix.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::rel_mix_song::Relation::Song.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
