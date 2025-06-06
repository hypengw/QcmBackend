use super::type_enum::ItemType;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "artist")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub native_id: String,
    pub name: String,
    #[serde(default)]
    pub sort_name: Option<String>,
    pub library_id: i64,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub album_count: i32,
    #[serde(default)]
    pub music_count: i32,
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
        has_many = "super::image::Entity",
        on_condition = r#"Expr::col(super::image::Column::ItemType).eq(ItemType::Artist)"#
    )]
    Image,
    #[sea_orm(
        has_many = "super::dynamic::Entity",
        on_condition = r#"Expr::col(super::dynamic::Column::ItemType).eq(ItemType::Artist)"#
    )]
    Dynamic,
    #[sea_orm(has_many = "super::rel_album_artist::Entity")]
    RelAlbum,
    #[sea_orm(has_many = "super::rel_song_artist::Entity")]
    RelSong,
}

impl Related<super::library::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Library.def()
    }
}
impl Related<super::image::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Image.def()
    }
}
impl Related<super::rel_album_artist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RelAlbum.def()
    }
}
impl Related<super::rel_song_artist::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::RelSong.def()
    }
}
impl Related<super::album::Entity> for Entity {
    fn to() -> RelationDef {
        super::rel_album_artist::Relation::Album.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::rel_album_artist::Relation::Artist.def().rev())
    }
}

impl Related<super::song::Entity> for Entity {
    fn to() -> RelationDef {
        super::rel_song_artist::Relation::Song.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::rel_song_artist::Relation::Artist.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
