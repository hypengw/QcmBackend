use crate::model as sqlm;
use sea_orm::prelude::*;
use sea_orm::{prelude::DateTimeUtc, DatabaseTransaction, EntityTrait};

pub async fn sync_drop_before(
    txn: &DatabaseTransaction,
    now: DateTimeUtc,
) -> Result<(), sea_orm::DbErr> {
    sqlm::album::Entity::delete_many()
        .filter(sqlm::album::Column::EditTime.lt(now))
        .exec(txn)
        .await?;
    Ok(())
}
