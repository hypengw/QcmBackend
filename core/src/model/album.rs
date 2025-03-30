use crate::db::values::StringVec;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "album")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub item_id: String,
    pub library_id: i64,
    pub name: String,
    pub pic_id: String,
    pub publish_time: DateTimeUtc,
    pub track_count: i32,
    pub description: String,
    pub company: String,
    pub type_: String,
    pub genres: StringVec,
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
}

impl Related<super::library::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Library.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl PartialEq for Column {
    fn eq(&self, other: &Self) -> bool {
        self.default_as_str() == other.default_as_str()
    }
}

