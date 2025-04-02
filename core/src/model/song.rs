use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "song")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub item_id: String,
    pub library_id: i64,
    pub name: String,
    #[sea_orm(nullable)]
    pub album_id: Option<i64>,
    pub track_number: i32,
    pub disc_number: i32,
    pub duration: f64,
    pub can_play: bool,
    pub popularity: f64,
    pub publish_time: DateTimeUtc,
    pub tags: Json,
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
    #[sea_orm(
        belongs_to = "super::album::Entity",
        from = "Column::AlbumId",
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

impl PartialEq for Column {
    fn eq(&self, other: &Self) -> bool {
        self.default_as_str() == other.default_as_str()
    }
}
