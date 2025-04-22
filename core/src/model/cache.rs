use super::type_enum::CacheType;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "cache")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    #[sea_orm(unique)]
    pub key: String,
    pub cache_type: CacheType,

    pub content_type: String,
    pub content_length: i64,

    #[serde(default)]
    pub blob: Option<Vec<u8>>,

    #[serde(default)]
    pub filename: Option<String>,

    #[serde(default = "chrono::Utc::now")]
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub timestamp: DateTimeUtc,

    #[serde(default = "chrono::Utc::now")]
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub last_use: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
