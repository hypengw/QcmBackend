use crate::model as sqlm;
use sea_orm::{prelude::DateTimeUtc, DatabaseTransaction, EntityTrait};
use sea_orm::{prelude::*, QuerySelect};

pub async fn sync_drop_before(
    txn: &DatabaseTransaction,
    provider_id: i64,
    now: DateTimeUtc,
) -> Result<(), sea_orm::DbErr> {
    let ids: Vec<i64> = sqlm::library::Entity::find()
        .select_only()
        .column(sqlm::library::Column::LibraryId)
        .filter(sqlm::library::Column::ProviderId.eq(provider_id))
        .into_tuple()
        .all(txn)
        .await?;

    sqlm::album::Entity::delete_many()
        .filter(sqlm::album::Column::EditTime.lt(now))
        .filter(sqlm::album::Column::LibraryId.is_in(ids))
        .exec(txn)
        .await?;
    Ok(())
}
