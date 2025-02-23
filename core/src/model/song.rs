use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "song")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub item_id: String,
    pub library_id: i32,
    pub name: String,
    pub album_id: String,
    pub track_number: i32,
    pub duration: DateTime,
    pub can_play: i32,
    pub cover_url: String,
    pub tags: String,
    pub popularity: f64,
    pub _edit_time: DateTime,
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
