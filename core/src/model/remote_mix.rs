use crate::db::values::Timestamp;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, DeriveEntityModel)]
#[sea_orm(table_name = "remote_mix")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub mix_id: i64,
    pub provider_id: i64,
    pub native_id: String,

    #[serde(default = "Timestamp::now")]
    #[sea_orm(default_expr = "Timestamp::now_expr()")]
    pub last_sync_at: Timestamp,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::provider::Entity",
        from = "Column::ProviderId",
        to = "super::provider::Column::ProviderId"
    )]
    Provider,
    #[sea_orm(
        belongs_to = "super::mix::Entity",
        from = "Column::MixId",
        to = "super::mix::Column::Id"
    )]
    Mix,
}

impl Related<super::provider::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Provider.def()
    }
}

impl Related<super::mix::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Mix.def()
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
