use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "provider")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub provider_id: i64,
    pub name: String,
    #[sea_orm(column_name = "type")]
    pub type_: String,
    pub cookie: String,
    pub custom: String,
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub edit_time: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
