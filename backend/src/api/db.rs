use qcm_core::model as sql_model;
use qcm_core::provider::Provider;
use sea_orm::{sea_query, DatabaseConnection, EntityTrait};
use std::sync::Arc;

use crate::error::ProcessError;

pub async fn add_provider(
    db: &DatabaseConnection,
    p: Arc<dyn Provider>,
) -> Result<i64, ProcessError> {
    let model = p.to_model();
    let r = sql_model::provider::Entity::insert(model)
        .on_conflict(
            sea_query::OnConflict::column(sql_model::provider::Column::ProviderId)
                .update_columns([
                    sql_model::provider::Column::Custom,
                    sql_model::provider::Column::Cookie,
                    sql_model::provider::Column::EditTime,
                ])
                .to_owned(),
        )
        .exec(db)
        .await?;
    Ok(r.last_insert_id)
}
