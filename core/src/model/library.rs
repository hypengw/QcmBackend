use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "library")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub library_id: i64,
    pub name: String,
    pub provider_id: i64,
    pub native_id: String,
    pub _edit_time: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::album::Entity")]
    Album,
    #[sea_orm(has_many = "super::artist::Entity")]
    Artist,
    #[sea_orm(has_many = "super::mix::Entity")]
    Mix,
    #[sea_orm(has_many = "super::radio::Entity")]
    Radio,
    #[sea_orm(has_many = "super::song::Entity")]
    Song,
    #[sea_orm(has_many = "super::program::Entity")]
    Program,
}

impl ActiveModelBehavior for ActiveModel {}
