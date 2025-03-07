use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "provider")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub provider_id: i64,
    pub name: String,
    pub auth: String,
    pub cookie: String,
    #[sea_orm(default_expr = "Expr::current_timestamp()")]
    pub edit_time: DateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
