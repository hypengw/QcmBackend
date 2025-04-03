use super::type_enum::ItemType;
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "cache")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub cache_id: i64,
    pub cache_type: ItemType,

    pub content_type: String,
    pub content_length: i64,
    pub timestamp: DateTimeUtc,
}
// unique: (cache_id, cache_type)

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
