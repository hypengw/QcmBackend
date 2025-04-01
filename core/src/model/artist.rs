use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "artist")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub item_id: String,
    pub name: String,
    pub library_id: i64,
    pub description: String,
    pub album_count: i32,
    pub music_count: i32,
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
}

impl Related<super::library::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Library.def()
    }
}

impl Related<super::album::Entity> for Entity {
    fn to() -> RelationDef {
        super::rel_album_artist::Relation::Album.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::rel_album_artist::Relation::Artist.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl PartialEq for Column {
    fn eq(&self, other: &Self) -> bool {
        self.default_as_str() == other.default_as_str()
    }
}
