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
    pub publish_time: DateTimeUtc,
    pub track_count: i32,
    pub description: String,
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

impl ActiveModelBehavior for ActiveModel {}