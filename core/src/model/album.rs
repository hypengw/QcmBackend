use super::type_enum::ItemType;
use super::util::epoch;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "album")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub native_id: String,
    pub library_id: i64,
    pub name: String,
    #[serde(default = "epoch")]
    pub publish_time: DateTimeUtc,
    pub track_count: i32,
    #[serde(default = "String::new")]
    pub description: String,
    #[serde(default = "String::new")]
    pub company: String,
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
    #[sea_orm(has_many = "super::song::Entity")]
    Song,
    #[sea_orm(
        has_many = "super::image::Entity",
        on_condition = r#"Expr::col(super::image::Column::ItemType).eq(ItemType::Album)"#
    )]
    Image,
}

impl Related<super::library::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Library.def()
    }
}

impl Related<super::song::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Song.def()
    }
}

impl Related<super::artist::Entity> for Entity {
    fn to() -> RelationDef {
        super::rel_album_artist::Relation::Artist.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::rel_album_artist::Relation::Album.def().rev())
    }
}

impl Related<super::image::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Image.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}