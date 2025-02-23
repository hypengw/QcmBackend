use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "mix_song")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub library_id: i32,
    pub song_id: String,
    pub mix_id: String,
    pub order_idx: i32,
    pub removed: i32,
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

impl ActiveModelBehavior for ActiveModel {}
