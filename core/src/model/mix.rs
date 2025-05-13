use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "mix")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub provider_id: i64,
    pub name: String,
    #[serde(default)]
    pub sort_name: Option<String>,
    pub native_id: String,
    pub track_count: i32,
    pub special_type: i32,
    pub description: String,
    // pub user_id: String,
    pub tags: Json,

    pub create_time: DateTimeUtc,
    pub update_time: DateTimeUtc,
    pub edit_time: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::provider::Entity",
        from = "Column::ProviderId",
        to = "super::provider::Column::ProviderId"
    )]
    Provider,
}

impl Related<super::library::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Provider.def()
    }
}

impl Related<super::song::Entity> for Entity {
    fn to() -> RelationDef {
        super::rel_mix_song::Relation::Song.def()
    }

    fn via() -> Option<RelationDef> {
        Some(super::rel_mix_song::Relation::Mix.def().rev())
    }
}

impl ActiveModelBehavior for ActiveModel {}
