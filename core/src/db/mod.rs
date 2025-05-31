pub mod sync;
pub mod values;
pub mod basic;

pub use basic::QueryBuilder;
pub use const_chunks::IteratorConstChunks;
use sea_orm::{
    sea_query::{self, IntoIden, IntoIndexColumn},
    ActiveModelTrait, DatabaseConnection, DatabaseTransaction, EntityTrait, InsertResult,
    TryInsertResult,
};
use strum::IntoEnumIterator;

pub fn columns_contains<Col>(list: &[Col], c: &Col) -> bool {
    use std::mem::discriminant;
    for l in list {
        if discriminant(l) == discriminant(c) {
            return true;
        }
    }
    return false;
}

pub struct DbOper {}
impl DbOper {
    pub async fn insert<Et, Col, A, I>(
        txn: &DatabaseTransaction,
        iter: I,
        conflict: &[Col],
        exclude: &[Col],
    ) -> Result<TryInsertResult<InsertResult<A>>, sea_orm::DbErr>
    where
        Et: EntityTrait,
        Col: IntoIden + Copy + IntoEnumIterator,
        A: ActiveModelTrait<Entity = Et>,
        I: IntoIterator<Item = A>,
    {
        Et::insert_many(iter)
            .on_conflict(
                sea_query::OnConflict::columns(conflict.iter().map(|c| c.clone()))
                    .update_columns(
                        Col::iter()
                            .filter(|e| {
                                !columns_contains(&conflict, e) && !columns_contains(&exclude, e)
                            })
                            .collect::<Vec<_>>(),
                    )
                    .to_owned(),
            )
            .on_empty_do_nothing()
            .exec(txn)
            .await
    }
    pub async fn insert_return_key<Et, Col, A, I>(
        txn: &DatabaseTransaction,
        iter: I,
        conflict: &[Col],
        exclude: &[Col],
    ) -> Result<
        TryInsertResult<
            Vec<<<Et as sea_orm::EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType>,
        >,
        sea_orm::DbErr,
    >
    where
        Et: EntityTrait,
        Et::Model: sea_orm::ModelTrait + sea_orm::IntoActiveModel<A>,
        Col: IntoIden + Copy + IntoEnumIterator,
        A: ActiveModelTrait<Entity = Et>,
        I: IntoIterator<Item = A>,
    {
        Et::insert_many(iter)
            .on_conflict(
                sea_query::OnConflict::columns(conflict.iter().map(|c| c.clone()))
                    .update_columns(
                        Col::iter()
                            .filter(|e| {
                                !columns_contains(&conflict, e) && !columns_contains(&exclude, e)
                            })
                            .collect::<Vec<_>>(),
                    )
                    .to_owned(),
            )
            .on_empty_do_nothing()
            .exec_with_returning_keys(txn)
            .await
    }
}

pub struct DbChunkOper<const N: usize> {}

impl<const N: usize> DbChunkOper<N> {
    pub async fn insert<Et, Col, A, I>(
        txn: &DatabaseTransaction,
        iter: I,
        conflict: &[Col],
        exclude: &[Col],
    ) -> Result<(), sea_orm::DbErr>
    where
        Et: EntityTrait,
        Col: IntoIden + Copy + IntoEnumIterator,
        A: ActiveModelTrait<Entity = Et>,
        I: IntoIterator<Item = A>,
    {
        let mut chunks = iter.into_iter().const_chunks::<N>();
        for iter in &mut chunks {
            DbOper::insert(&txn, iter, &conflict, &exclude).await?;
        }

        if let Some(rm) = chunks.into_remainder() {
            DbOper::insert(&txn, rm.into_iter(), &conflict, &exclude).await?;
        }
        Ok(())
    }

    pub async fn insert_return_key<Et, Col, A, I>(
        txn: &DatabaseTransaction,
        iter: I,
        conflict: &[Col],
        exclude: &[Col],
    ) -> Result<
        Vec<<<Et as sea_orm::EntityTrait>::PrimaryKey as sea_orm::PrimaryKeyTrait>::ValueType>,
        sea_orm::DbErr,
    >
    where
        Et: EntityTrait,
        Et::Model: sea_orm::ModelTrait + sea_orm::IntoActiveModel<A>,
        Col: IntoIden + Copy + IntoEnumIterator,
        A: ActiveModelTrait<Entity = Et>,
        I: IntoIterator<Item = A>,
    {
        let mut out = Vec::new();
        let mut chunks = iter.into_iter().const_chunks::<N>();
        for iter in &mut chunks {
            match DbOper::insert_return_key(&txn, iter, &conflict, &exclude).await? {
                TryInsertResult::Inserted(inserted) => {
                    out.extend(inserted);
                }
                _ => {
                    return Err(sea_orm::DbErr::RecordNotInserted);
                }
            }
        }

        if let Some(rm) = chunks.into_remainder() {
            match DbOper::insert_return_key(&txn, rm.into_iter(), &conflict, &exclude).await? {
                TryInsertResult::Inserted(inserted) => {
                    out.extend(inserted);
                }
                _ => {
                    return Err(sea_orm::DbErr::RecordNotInserted);
                }
            }
        }
        Ok(out)
    }
}

