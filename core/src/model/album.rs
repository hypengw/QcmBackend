use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "album")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub item_id: String,
    pub library_id: i32,
    pub name: String,
    pub pic_url: String,
    pub publish_time: DateTime,
    pub track_count: i32,
    pub description: String,
    pub company: String,
    pub type_: String,
    pub _edit_time: DateTime,
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
