use super::type_enum::{ImageType, ItemType};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "image")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub item_id: i64,
    pub item_type: ItemType,
    pub image_type: ImageType,

    pub library_id: i64, // foreign

    #[serde(default)]
    pub native_id: Option<String>,

    #[serde(default)]
    pub db: Option<String>,

    #[serde(default)]
    pub fresh: String, // custom string for testing changed
    #[serde(default = "chrono::Utc::now")]
    pub timestamp: DateTimeUtc,
}
// unique: (item_id, item_type, image_type)

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
        from = "Column::ItemId",
        to = "super::album::Column::Id"
    )]
    Album,
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

impl ActiveModelBehavior for ActiveModel {}
