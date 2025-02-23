use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "playlist")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub item_id: String,
    pub library_id: i32,
    pub name: String,
    pub pic_url: String,
    pub track_count: i32,
    pub special_type: i32,
    pub description: String,
    pub create_time: DateTime,
    pub update_time: DateTime,
    pub play_count: i32,
    pub user_id: String,
    pub tags: String,
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
