use super::basic::QueryBuilder;
use super::DbChunkOper;
use crate::model as sqlm;
use sea_orm::sea_query::{DeleteStatement, SqliteQueryBuilder};
use sea_orm::{prelude::DateTimeUtc, DatabaseTransaction, EntityTrait};
use sea_orm::{prelude::*, QuerySelect, Statement};
use sea_orm::{sea_query, Condition};

pub async fn sync_drop_before(
    txn: &DatabaseTransaction,
    provider_id: i64,
    now: DateTimeUtc,
) -> Result<(), sea_orm::DbErr> {
    use sea_query::{Alias, Asterisk, CommonTableExpression, Expr, Query, WithClause};

    // clean library first
    sqlm::library::Entity::delete_many()
        .filter(sqlm::library::Column::ProviderId.eq(provider_id))
        .filter(sqlm::library::Column::EditTime.lt(now))
        .exec(txn)
        .await?;

    let ids: Vec<i64> = sqlm::library::Entity::find()
        .select_only()
        .column(sqlm::library::Column::LibraryId)
        .filter(sqlm::library::Column::ProviderId.eq(provider_id))
        .into_tuple()
        .all(txn)
        .await?;

    {
        let cte = CommonTableExpression::new()
            .query(
                Query::select()
                    .column(sqlm::rel_album_artist::Column::AlbumId)
                    .from(sqlm::rel_album_artist::Entity)
                    .inner_join(
                        sqlm::album::Entity,
                        Condition::all()
                            .add(
                                Expr::col((
                                    sqlm::rel_album_artist::Entity,
                                    sqlm::rel_album_artist::Column::AlbumId,
                                ))
                                .equals((sqlm::album::Entity, sqlm::album::Column::Id)),
                            )
                            .add(
                                Expr::col((sqlm::album::Entity, sqlm::album::Column::LibraryId))
                                    .is_in(ids.clone()),
                            ),
                    )
                    .and_where(
                        Expr::col((
                            sqlm::rel_album_artist::Entity,
                            sqlm::rel_album_artist::Column::EditTime,
                        ))
                        .lt(now),
                    )
                    .to_owned(),
            )
            .columns([sqlm::rel_album_artist::Column::AlbumId])
            .table_name("to_delete")
            .to_owned();

        let with = WithClause::new().cte(cte).to_owned();

        let delete = Query::delete()
            .from_table(sqlm::rel_album_artist::Entity)
            .and_where(
                Expr::col((
                    sqlm::rel_album_artist::Entity,
                    sqlm::rel_album_artist::Column::AlbumId,
                ))
                .in_subquery(
                    Query::select()
                        .column(sqlm::rel_album_artist::Column::AlbumId)
                        .from("to_delete")
                        .to_owned(),
                ),
            )
            .to_owned();

        let query = delete.with(with);

        let stmt = query.build(QueryBuilder);

        txn.execute(Statement::from_sql_and_values(
            txn.get_database_backend(),
            stmt.0,
            stmt.1,
        ))
        .await?;
    }

    {
        let cte = CommonTableExpression::new()
            .query(
                Query::select()
                    .column(sqlm::rel_song_artist::Column::SongId)
                    .from(sqlm::rel_song_artist::Entity)
                    .inner_join(
                        sqlm::song::Entity,
                        Condition::all()
                            .add(
                                Expr::col((
                                    sqlm::rel_song_artist::Entity,
                                    sqlm::rel_song_artist::Column::SongId,
                                ))
                                .equals((sqlm::song::Entity, sqlm::song::Column::Id)),
                            )
                            .add(
                                Expr::col((sqlm::song::Entity, sqlm::song::Column::LibraryId))
                                    .is_in(ids.clone()),
                            ),
                    )
                    .and_where(
                        Expr::col((
                            sqlm::rel_song_artist::Entity,
                            sqlm::rel_song_artist::Column::EditTime,
                        ))
                        .lt(now),
                    )
                    .to_owned(),
            )
            .columns([sqlm::rel_song_artist::Column::SongId])
            .table_name("to_delete")
            .to_owned();

        let with = WithClause::new().cte(cte).to_owned();

        let delete = Query::delete()
            .from_table(sqlm::rel_song_artist::Entity)
            .and_where(
                Expr::col((
                    sqlm::rel_song_artist::Entity,
                    sqlm::rel_song_artist::Column::SongId,
                ))
                .in_subquery(
                    Query::select()
                        .column(sqlm::rel_song_artist::Column::SongId)
                        .from("to_delete")
                        .to_owned(),
                ),
            )
            .to_owned();

        let query = delete.with(with);

        let stmt = query.build(QueryBuilder);

        txn.execute(Statement::from_sql_and_values(
            txn.get_database_backend(),
            stmt.0,
            stmt.1,
        ))
        .await?;
    }

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

    sqlm::mix::Entity::delete_many()
        .filter(sqlm::mix::Column::EditTime.lt(now))
        .filter(sqlm::mix::Column::ProviderId.eq(provider_id))
        .exec(txn)
        .await?;

    Ok(())
}

pub async fn sync_song_artist_ids(
    txn: &DatabaseTransaction,
    library_id: i64,
    ids: Vec<(String, String)>,
) -> Result<(), sea_orm::DbErr> {
    if ids.is_empty() {
        return Ok(());
    }
    let now = chrono::Utc::now();

    let conflict = [
        sqlm::rel_song_artist::Column::SongId,
        sqlm::rel_song_artist::Column::ArtistId,
    ];
    use sea_query::{Alias, Asterisk, CommonTableExpression, Expr, Query, WithClause};

    let with_clause = WithClause::new()
        .cte(
            CommonTableExpression::new()
                .query(
                    Query::select()
                        .column(Asterisk)
                        .from_values(ids, Alias::new("input"))
                        .to_owned(),
                )
                .columns([Alias::new("song_item_id"), Alias::new("artist_item_id")])
                .table_name(Alias::new("item_id_map"))
                .to_owned(),
        )
        .to_owned();

    let relations = sea_query::Query::select()
        .expr(Expr::col((sqlm::song::Entity, sqlm::song::Column::Id)))
        .expr(Expr::col((sqlm::artist::Entity, sqlm::artist::Column::Id)))
        .expr(Expr::value(now))
        .from(sqlm::song::Entity)
        .inner_join(
            Alias::new("item_id_map"),
            Expr::col((sqlm::song::Entity, sqlm::song::Column::NativeId))
                .equals((Alias::new("item_id_map"), Alias::new("song_item_id"))),
        )
        .inner_join(
            sqlm::artist::Entity,
            Condition::all()
                .add(
                    Expr::col((sqlm::artist::Entity, sqlm::artist::Column::NativeId))
                        .equals((Alias::new("item_id_map"), Alias::new("artist_item_id"))),
                )
                .add(
                    Expr::col((sqlm::artist::Entity, sqlm::artist::Column::LibraryId))
                        .eq(library_id),
                ),
        )
        .and_where(Expr::col((sqlm::song::Entity, sqlm::song::Column::LibraryId)).eq(library_id))
        .to_owned();

    let stmt = sea_query::Query::insert()
        .into_table(sqlm::rel_song_artist::Entity)
        .columns([
            sqlm::rel_song_artist::Column::SongId,
            sqlm::rel_song_artist::Column::ArtistId,
            sqlm::rel_song_artist::Column::EditTime,
        ])
        .select_from(relations)
        .unwrap()
        .on_conflict(
            sea_query::OnConflict::columns(conflict)
                .update_column(sqlm::rel_song_artist::Column::EditTime)
                .to_owned(),
        )
        .to_owned()
        .with(with_clause)
        .to_owned();

    let builder = txn.get_database_backend();
    txn.execute(builder.build(&stmt)).await?;

    Ok(())
}

pub async fn sync_album_artist_ids(
    txn: &DatabaseTransaction,
    library_id: i64,
    ids: Vec<(String, String)>,
) -> Result<(), sea_orm::DbErr> {
    if ids.is_empty() {
        return Ok(());
    }
    let now = chrono::Utc::now();

    use sea_query::{Alias, Asterisk, CommonTableExpression, Expr, Query, WithClause};
    //let al_item_id_map = Alias::new("item_id_map");

    let conflict = [
        sqlm::rel_album_artist::Column::AlbumId,
        sqlm::rel_album_artist::Column::ArtistId,
    ];

    let with_clause = WithClause::new()
        .cte(
            CommonTableExpression::new()
                .query(
                    Query::select()
                        .column(Asterisk)
                        .from_values(ids, Alias::new("input"))
                        .to_owned(),
                )
                .columns([Alias::new("album_item_id"), Alias::new("artist_item_id")])
                .table_name(Alias::new("item_id_map"))
                .to_owned(),
        )
        .to_owned();

    let relations = sea_query::Query::select()
        .expr(Expr::col((sqlm::album::Entity, sqlm::album::Column::Id)))
        .expr(Expr::col((sqlm::artist::Entity, sqlm::artist::Column::Id)))
        .expr(Expr::value(now))
        .from(sqlm::album::Entity)
        .inner_join(
            Alias::new("item_id_map"),
            Expr::col((sqlm::album::Entity, sqlm::album::Column::NativeId))
                .equals((Alias::new("item_id_map"), Alias::new("album_item_id"))),
        )
        .inner_join(
            sqlm::artist::Entity,
            Condition::all()
                .add(
                    Expr::col((sqlm::artist::Entity, sqlm::artist::Column::NativeId))
                        .equals((Alias::new("item_id_map"), Alias::new("artist_item_id"))),
                )
                .add(
                    Expr::col((sqlm::artist::Entity, sqlm::artist::Column::LibraryId))
                        .eq(library_id),
                ),
        )
        .and_where(Expr::col((sqlm::album::Entity, sqlm::album::Column::LibraryId)).eq(library_id))
        .to_owned();

    let stmt = sea_query::Query::insert()
        .into_table(sqlm::rel_album_artist::Entity)
        .columns([
            sqlm::rel_album_artist::Column::AlbumId,
            sqlm::rel_album_artist::Column::ArtistId,
            sqlm::rel_album_artist::Column::EditTime,
        ])
        .select_from(relations)
        .unwrap()
        .on_conflict(
            sea_query::OnConflict::columns(conflict)
                .update_column(sqlm::rel_album_artist::Column::EditTime)
                .to_owned(),
        )
        .to_owned()
        .with(with_clause)
        .to_owned();
    let builder = txn.get_database_backend();
    txn.execute(builder.build(&stmt)).await?;

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

pub async fn sync_dynamic_items<I>(txn: &DatabaseTransaction, item_commons: I) -> Result<(), DbErr>
where
    I: IntoIterator<Item = sqlm::dynamic::ActiveModel>,
{
    let conflict = [
        sqlm::dynamic::Column::ItemId,
        sqlm::dynamic::Column::ItemType,
    ];
    let exclude = [sqlm::dynamic::Column::Id];
    DbChunkOper::<50>::insert(txn, item_commons, &conflict, &exclude).await?;

    Ok(())
}
