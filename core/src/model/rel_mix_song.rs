use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "rel_mix_song")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub song_id: i64,
    pub mix_id: i64,
    pub order_idx: i64,
    pub edit_time: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::mix::Entity",
        from = "Column::MixId",
        to = "super::mix::Column::Id"
    )]
    Mix,
    #[sea_orm(
        belongs_to = "super::song::Entity",
        from = "Column::SongId",
        to = "super::song::Column::Id"
    )]
    Song,
}

impl Related<super::mix::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Mix.def()
    }
}

impl Related<super::song::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Song.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
