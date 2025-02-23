use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "collection")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub library_id: i32,
    pub user_id: String,
    pub type_: String,
    pub item_id: String,
    pub collect_time: DateTime,
    pub removed: i32,
}

#[derive(Copy, Clone, Debug, EnumIter)]
pub enum Relation {
    Library,
}

impl RelationTrait for Relation {
    fn def(&self) -> RelationDef {
        match self {
            Self::Library => Entity::belongs_to(super::library::Entity)
                .from(Column::LibraryId)
                .to(super::library::Column::LibraryId)
                .into(),
        }
    }
}

impl ActiveModelBehavior for ActiveModel {}
