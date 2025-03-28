use crate::db::values::StringVec;
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "cache")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub library_id: i64,
    pub key: String,
    pub content_type: String,
    pub content_length: i64,
    pub content: Option<Vec<u8>>,
    pub timestamp: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
