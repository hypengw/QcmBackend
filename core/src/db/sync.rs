use crate::model as sqlm;
use sea_orm::sea_query;
use sea_orm::{prelude::DateTimeUtc, DatabaseTransaction, EntityTrait};
use sea_orm::{prelude::*, QuerySelect, Statement};

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
        .filter(sqlm::album::Column::LibraryId.is_in(ids.clone()))
        .exec(txn)
        .await?;

    sqlm::song::Entity::delete_many()
        .filter(sqlm::song::Column::EditTime.lt(now))
        .filter(sqlm::song::Column::LibraryId.is_in(ids.clone()))
        .exec(txn)
        .await?;

    sqlm::artist::Entity::delete_many()
        .filter(sqlm::artist::Column::EditTime.lt(now))
        .filter(sqlm::artist::Column::LibraryId.is_in(ids))
        .exec(txn)
        .await?;

    Ok(())
}

pub async fn sync_song_album_ids(
    txn: &DatabaseTransaction,
    library_id: i64,
    ids: Vec<(String, String)>,
) -> Result<(), sea_orm::DbErr> {

    if ids.is_empty() {
        return Ok(());
    }

    use sea_query::{Alias, Asterisk, CommonTableExpression, Expr, Query, WithClause};
    let al_item_id_map = Alias::new("item_id_map");

    let relations = sea_query::Query::select()
        .expr(Expr::col((sqlm::song::Entity, sqlm::song::Column::Id)))
        .expr(Expr::col((sqlm::album::Entity, sqlm::album::Column::Id)))
        .from(sqlm::song::Entity)
        .inner_join(
            al_item_id_map.clone(),
            Expr::col((sqlm::song::Entity, sqlm::song::Column::NativeId))
                .equals((al_item_id_map.clone(), Alias::new("song_item_id"))),
        )
        .inner_join(
            sqlm::album::Entity,
            Expr::col((sqlm::album::Entity, sqlm::album::Column::NativeId))
                .equals((al_item_id_map.clone(), Alias::new("album_item_id"))),
        )
        .and_where(Expr::col((sqlm::song::Entity, sqlm::song::Column::LibraryId)).eq(library_id))
        .to_owned();

    let with_clause = WithClause::new()
        .cte(
            CommonTableExpression::new()
                .query(
                    Query::select()
                        .column(Asterisk)
                        .from_values(ids, Alias::new("input"))
                        .to_owned(),
                )
                .columns([Alias::new("song_item_id"), Alias::new("album_item_id")])
                .table_name(al_item_id_map.clone())
                .to_owned(),
        )
        .cte(
            CommonTableExpression::new()
                .query(relations)
                .columns([Alias::new("id"), Alias::new("album_id")])
                .table_name(Alias::new("res"))
                .to_owned(),
        )
        .to_owned();

    let builder = txn.get_database_backend();
    let stmt = sea_query::Query::update()
        .table(sqlm::song::Entity)
        .value(
            sqlm::song::Column::AlbumId,
            Expr::col((Alias::new("res"), Alias::new("album_id"))),
        )
        .to_owned()
        .with(with_clause)
        .to_owned();

    let raw = format!(
        "
                {}
                FROM res
                WHERE res.id = song.id
                ",
        builder.build(&stmt).to_string()
    );
    txn.execute(Statement::from_string(builder, raw)).await?;
    Ok(())
}
