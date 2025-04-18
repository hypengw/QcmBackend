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
    pub library_id: i64,
    pub description: String,
    pub album_count: i32,
    pub music_count: i32,
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
