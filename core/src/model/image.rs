use super::type_enum::{ImageType, ItemType};
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "image")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub item_id: i64,
    pub item_type: ItemType,
    pub image_type: ImageType,

    pub library_id: i64, // foreign

    pub item_native_id: String,
    pub native_id: Option<String>,

    pub db: Option<String>,

    pub fresh: String, // custom string for testing changed
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
}

impl Related<super::library::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Library.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
