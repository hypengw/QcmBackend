use crate::msg::{
    self,
    filter::{
        AlbumFilter, ArtistFilter, DateCondition, IntCondition, StringCondition, TypeCondition,
    },
};
use chrono::TimeZone;
use qcm_core::{db::values::Timestamp, model as sqlm, plugin::Plugin};
use sea_orm::{
    sea_query::{Expr, SimpleExpr},
    Condition,
};
use sea_orm::{QueryFilter, RelationTrait};
use uuid::fmt::Simple;

pub trait SelectQcmMsgFilters: Sized {
    type Filter;
    fn qcm_filters<'a, I>(self, filters: I) -> Self
    where
        I: IntoIterator<Item = &'a Self::Filter>,
        Self::Filter: 'a;
}

impl SelectQcmMsgFilters for sea_orm::Select<sqlm::album::Entity> {
    type Filter = AlbumFilter;

    fn qcm_filters<'a, I>(self, filters: I) -> Self
    where
        I: IntoIterator<Item = &'a Self::Filter>,
        Self::Filter: 'a,
    {
        use msg::filter::album_filter::Payload;
        use sea_orm::sea_query::{Expr, Query, SelectStatement};
        let mut select = self;

        for f in filters {
            let expr = match &f.payload {
                Some(Payload::ArtistNameFilter(artist)) => {
                    artist
                        .get_expr(
                            Expr::col((sqlm::artist::Entity, sqlm::artist::Column::Name)).into(),
                        )
                        .map(|artist_name_expr| {
                            let subquery: SelectStatement = Query::select()
                                .expr(Expr::val(1)) // SELECT 1
                                .from(sqlm::song::Entity)
                                .inner_join(
                                    sqlm::rel_song_artist::Entity,
                                    sqlm::rel_song_artist::Relation::Song.def(),
                                )
                                .inner_join(
                                    sqlm::artist::Entity,
                                    sqlm::artist::Relation::RelSong.def(),
                                )
                                .and_where(
                                    Expr::col(sqlm::song::Column::AlbumId)
                                        .equals((sqlm::album::Entity, sqlm::album::Column::Id)),
                                )
                                .and_where(artist_name_expr)
                                .limit(1)
                                .to_owned();
                            Expr::exists(subquery)
                        })
                }
                Some(Payload::AlbumArtistIdFilter(id)) => id
                    .get_expr(Expr::col((sqlm::artist::Entity, sqlm::artist::Column::Id)).into())
                    .map(|expr| {
                        let subquery: SelectStatement = Query::select()
                            .expr(Expr::val(1)) // SELECT 1
                            .from(sqlm::artist::Entity)
                            .inner_join(
                                sqlm::rel_album_artist::Entity,
                                sqlm::rel_album_artist::Relation::Artist.def(),
                            )
                            .and_where(
                                Expr::col((
                                    sqlm::rel_album_artist::Entity,
                                    sqlm::rel_album_artist::Column::AlbumId,
                                ))
                                .equals((sqlm::album::Entity, sqlm::album::Column::Id)),
                            )
                            .and_where(expr)
                            .limit(1)
                            .to_owned();
                        Expr::exists(subquery)
                    }),
                Some(Payload::ArtistIdFilter(id)) => id
                    .get_expr(
                        Expr::col((
                            sqlm::rel_song_artist::Entity,
                            sqlm::rel_song_artist::Column::ArtistId,
                        ))
                        .into(),
                    )
                    .map(|expr| {
                        let subquery: SelectStatement = Query::select()
                            .expr(Expr::val(1)) // SELECT 1
                            .from(sqlm::song::Entity)
                            .inner_join(
                                sqlm::rel_song_artist::Entity,
                                sqlm::rel_song_artist::Relation::Song.def(),
                            )
                            .and_where(
                                Expr::col((sqlm::song::Entity, sqlm::song::Column::AlbumId))
                                    .equals((sqlm::album::Entity, sqlm::album::Column::Id)),
                            )
                            .and_where(expr)
                            .limit(1)
                            .to_owned();
                        Expr::exists(subquery)
                    }),
                Some(Payload::TitleFilter(title)) => {
                    title.get_expr_from_col(sqlm::album::Column::Name)
                }
                Some(Payload::TrackFilter(track)) => {
                    track.get_expr_from_col(sqlm::album::Column::TrackCount)
                }
                Some(Payload::DurationFilter(duration)) => {
                    duration.get_expr_from_col(sqlm::album::Column::Duration)
                }
                Some(Payload::YearFilter(year)) => {
                    year.get_expr_from_col(sqlm::album::Column::PublishTime)
                }
                Some(Payload::AddedDateFilter(added)) => {
                    added.get_expr_from_col(sqlm::album::Column::AddedAt)
                }
                Some(Payload::TypeFilter(album_type)) => {
                    album_type.get_expr_from_col(sqlm::album::Column::Type)
                }
                Some(Payload::DiscCountFilter(disc_count)) => {
                    disc_count.get_expr_from_col(sqlm::album::Column::DiscCount)
                }
                Some(Payload::LastPlayedAtFilter(last_played_at)) => last_played_at.get_expr(
                    Expr::col((sqlm::dynamic::Entity, sqlm::dynamic::Column::LastPlayedAt)),
                ),
                None => None,
            };

            if let Some(expr) = expr {
                select = select.filter(expr);
            }
        }
        select
    }
}

impl SelectQcmMsgFilters for sea_orm::Select<sqlm::artist::Entity> {
    type Filter = ArtistFilter;

    fn qcm_filters<'a, I>(self, filters: I) -> Self
    where
        I: IntoIterator<Item = &'a Self::Filter>,
        Self::Filter: 'a,
    {
        use msg::filter::artist_filter::Payload;
        use sea_orm::sea_query::{Expr, Query, SelectStatement};
        let mut select = self;

        for f in filters {
            let expr = match &f.payload {
                Some(Payload::NameFilter(name)) => {
                    name.get_expr_from_col(sqlm::artist::Column::Name)
                }
                Some(Payload::AddedDateFilter(added)) => {
                    added.get_expr_from_col(sqlm::artist::Column::AddedAt)
                }
                Some(Payload::AlbumTitleFilter(album_name)) => {
                    album_name
                        .get_expr(
                            Expr::col((sqlm::album::Entity, sqlm::album::Column::Name)).into(),
                        )
                        .map(|album_name_expr| {
                            let subquery: SelectStatement = Query::select()
                                .expr(Expr::val(1)) // SELECT 1
                                .from(sqlm::album::Entity)
                                .inner_join(
                                    sqlm::rel_album_artist::Entity,
                                    sqlm::rel_album_artist::Relation::Album.def(),
                                )
                                .and_where(
                                    Expr::col((sqlm::artist::Entity, sqlm::artist::Column::Id))
                                        .equals((
                                            sqlm::rel_album_artist::Entity,
                                            sqlm::rel_album_artist::Column::ArtistId,
                                        )),
                                )
                                .and_where(album_name_expr)
                                .limit(1)
                                .to_owned();
                            Expr::exists(subquery)
                        })
                }
                Some(_) => None::<SimpleExpr>,
                None => None,
            };

            if let Some(expr) = expr {
                select = select.filter(expr);
            }
        }
        select
    }
}

impl SelectQcmMsgFilters for sea_orm::Select<sqlm::mix::Entity> {
    type Filter = msg::filter::MixFilter;

    fn qcm_filters<'a, I>(self, filters: I) -> Self
    where
        I: IntoIterator<Item = &'a Self::Filter>,
        Self::Filter: 'a,
    {
        use msg::filter::mix_filter::Payload;
        let mut select = self;

        for f in filters {
            let expr = match &f.payload {
                Some(Payload::NameFilter(name)) => name.get_expr_from_col(sqlm::mix::Column::Name),
                Some(Payload::TrackFilter(track)) => {
                    track.get_expr_from_col(sqlm::mix::Column::TrackCount)
                }
                Some(Payload::AddedDateFilter(added)) => {
                    added.get_expr_from_col(sqlm::mix::Column::AddedAt)
                }
                Some(_) => None::<SimpleExpr>,
                None => None,
            };

            if let Some(expr) = expr {
                select = select.filter(expr);
            }
        }
        select
    }
}

impl SelectQcmMsgFilters for sea_orm::Select<sqlm::remote_mix::Entity> {
    type Filter = msg::filter::RemoteMixFilter;

    fn qcm_filters<'a, I>(self, filters: I) -> Self
    where
        I: IntoIterator<Item = &'a Self::Filter>,
        Self::Filter: 'a,
    {
        use msg::filter::remote_mix_filter::Payload;
        let mut select = self;

        for f in filters {
            let expr = match &f.payload {
                Some(Payload::NameFilter(name)) => {
                    name.get_expr_from_col(sqlm::remote_mix::Column::Name)
                }
                Some(Payload::TypeFilter(mix_type)) => mix_type.get_expr(Expr::col((
                    sqlm::remote_mix::Entity,
                    sqlm::remote_mix::Column::MixType,
                ))),
                Some(Payload::TrackFilter(track)) => {
                    track.get_expr_from_col(sqlm::remote_mix::Column::TrackCount)
                }
                None => None,
            };

            if let Some(expr) = expr {
                select = select.filter(expr);
            }
        }
        select
    }
}

trait StringFilterTrait {
    fn get_condition(&self) -> StringCondition;
    fn get_value(&self) -> &str;

    fn get_expr_from_col<C>(&self, col: C) -> Option<SimpleExpr>
    where
        C: sea_orm::ColumnTrait,
    {
        self.get_expr(Expr::col(col))
    }
    fn get_expr(&self, col: Expr) -> Option<SimpleExpr> {
        string_condition_to_expr(col, self.get_condition(), self.get_value())
    }
}

trait IntFilterTrait {
    fn get_condition(&self) -> IntCondition;
    fn get_value(&self) -> i64;

    fn get_expr_from_col<C>(&self, col: C) -> Option<SimpleExpr>
    where
        C: sea_orm::ColumnTrait,
    {
        self.get_expr(Expr::col(col))
    }
    fn get_expr(&self, col: Expr) -> Option<SimpleExpr> {
        return int_condition_to_expr(col, self.get_condition(), self.get_value());
    }
}
trait TypeFilterTrait {
    fn get_condition(&self) -> TypeCondition;
    fn get_value(&self) -> i64;

    fn get_expr_from_col<C>(&self, col: C) -> Option<SimpleExpr>
    where
        C: sea_orm::ColumnTrait,
    {
        self.get_expr(Expr::col(col))
    }
    fn get_expr(&self, col: Expr) -> Option<SimpleExpr> {
        return type_condition_to_expr(col, self.get_condition(), self.get_value());
    }
}

trait TypeStringFilterTrait {
    fn get_condition(&self) -> TypeCondition;
    fn get_value(&self) -> &str;

    fn get_expr_from_col<C>(&self, col: C) -> Option<SimpleExpr>
    where
        C: sea_orm::ColumnTrait,
    {
        self.get_expr(Expr::col(col))
    }
    fn get_expr(&self, col: Expr) -> Option<SimpleExpr> {
        return type_string_condition_to_expr(col, self.get_condition(), self.get_value());
    }
}

trait IdFilterTrait {
    fn get_value(&self) -> i64;

    fn get_expr_from_col<C>(&self, col: C) -> Option<SimpleExpr>
    where
        C: sea_orm::ColumnTrait,
    {
        self.get_expr(Expr::col(col))
    }
    fn get_expr(&self, col: Expr) -> Option<SimpleExpr> {
        Some(col.eq(self.get_value()))
    }
}

trait DateFilterTrait {
    fn get_condition(&self) -> DateCondition;
    fn get_value(&self) -> Timestamp;

    fn get_expr_from_col<C>(&self, col: C) -> Option<SimpleExpr>
    where
        C: sea_orm::ColumnTrait,
    {
        self.get_expr(Expr::col(col))
    }
    fn get_expr(&self, col: Expr) -> Option<SimpleExpr> {
        return date_condition_to_expr(col, self.get_condition(), self.get_value());
    }
}

macro_rules! impl_int_filter {
    ($ty:ty) => {
        impl IntFilterTrait for $ty {
            fn get_condition(&self) -> IntCondition {
                self.condition()
            }
            fn get_value(&self) -> i64 {
                self.value as i64
            }
        }
    };
}
macro_rules! impl_type_filter {
    ($ty:ty) => {
        impl TypeFilterTrait for $ty {
            fn get_condition(&self) -> TypeCondition {
                self.condition()
            }
            fn get_value(&self) -> i64 {
                self.value as i64
            }
        }
    };
}
macro_rules! impl_date_filter {
    ($ty:ty) => {
        impl DateFilterTrait for $ty {
            fn get_condition(&self) -> DateCondition {
                self.condition()
            }
            fn get_value(&self) -> Timestamp {
                Timestamp::from_millis(self.value)
            }
        }
    };
}
macro_rules! impl_string_filter {
    ($ty:ty) => {
        impl StringFilterTrait for $ty {
            fn get_condition(&self) -> StringCondition {
                self.condition()
            }
            fn get_value(&self) -> &str {
                &self.value
            }
        }
    };
}
macro_rules! impl_type_string_filter {
    ($ty:ty) => {
        impl TypeStringFilterTrait for $ty {
            fn get_condition(&self) -> TypeCondition {
                self.condition()
            }
            fn get_value(&self) -> &str {
                &self.value
            }
        }
    };
}
macro_rules! impl_id_filter {
    ($ty:ty) => {
        impl IdFilterTrait for $ty {
            fn get_value(&self) -> i64 {
                self.value
            }
        }
    };
}

impl_int_filter!(msg::filter::TrackCountFilter);
impl_int_filter!(msg::filter::DurationFilter);
impl_string_filter!(msg::filter::NameFilter);
impl_string_filter!(msg::filter::TitleFilter);
impl_string_filter!(msg::filter::ArtistNameFilter);
impl_string_filter!(msg::filter::AlbumTitleFilter);
impl_id_filter!(msg::filter::ArtistIdFilter);
impl_id_filter!(msg::filter::AlbumArtistIdFilter);
impl_date_filter!(msg::filter::AddedDateFilter);
impl_date_filter!(msg::filter::LastPlayedAtFilter);
impl_type_filter!(msg::filter::TypeFilter);
impl_type_string_filter!(msg::filter::TypeStringFilter);
impl_int_filter!(msg::filter::DiscCountFilter);

impl IntFilterTrait for msg::filter::YearFilter {
    fn get_condition(&self) -> IntCondition {
        self.condition()
    }
    fn get_value(&self) -> i64 {
        self.value as i64
    }
    fn get_expr(&self, col: Expr) -> Option<SimpleExpr> {
        use chrono::Utc;
        let dt: Timestamp = Utc
            .with_ymd_and_hms(self.value, 1, 1, 0, 0, 0)
            .unwrap()
            .into();
        match self.condition() {
            IntCondition::Equal => {
                // we need to compare instead of equal
                let dt_next: Timestamp = Utc
                    .with_ymd_and_hms(self.value + 1, 1, 1, 0, 0, 0)
                    .unwrap()
                    .into();
                Some(
                    Condition::all()
                        .add(col.clone().gte(dt.clone()))
                        .add(col.lt(dt_next.clone()))
                        .into(),
                )
            }
            IntCondition::EqualNot => {
                let dt_next = Utc
                    .with_ymd_and_hms(self.value + 1, 1, 1, 0, 0, 0)
                    .unwrap()
                    .to_string();
                Some(
                    Condition::all()
                        .add(col.clone().lt(dt.clone()))
                        .add(col.gt(dt_next.clone()))
                        .into(),
                )
            }
            IntCondition::Greater => Some(col.gt(dt)),
            IntCondition::GreaterEqual => Some(col.gte(dt)),
            IntCondition::Less => Some(col.lt(dt)),
            IntCondition::LessEqual => Some(col.lte(dt)),
            IntCondition::Unspecified => None,
        }
    }
}

pub fn int_condition_to_expr(col: Expr, cond: IntCondition, val: i64) -> Option<SimpleExpr> {
    match cond {
        IntCondition::Equal => Some(col.eq(val)),
        IntCondition::EqualNot => Some(col.ne(val)),
        IntCondition::Greater => Some(col.gt(val)),
        IntCondition::GreaterEqual => Some(col.gte(val)),
        IntCondition::Less => Some(col.lt(val)),
        IntCondition::LessEqual => Some(col.lte(val)),
        IntCondition::Unspecified => None,
    }
}

pub fn string_condition_to_expr(col: Expr, cond: StringCondition, s: &str) -> Option<SimpleExpr> {
    match cond {
        StringCondition::Contains => {
            let s = format!("%{}%", s);
            Some(col.like(s))
        }
        StringCondition::ContainsNot => {
            let s = format!("%{}%", s);
            Some(col.not_like(s))
        }
        StringCondition::Is => Some(col.eq(s)),
        StringCondition::IsNot => Some(col.ne(s)),
        StringCondition::Unspecified => None,
    }
}

pub fn date_condition_to_expr(
    col: Expr,
    cond: DateCondition,
    val: Timestamp,
) -> Option<SimpleExpr> {
    match cond {
        DateCondition::After => Some(col.gte(val)),
        DateCondition::Before => Some(col.lte(val)),
        DateCondition::Null => Some(col.is_null()),
        DateCondition::NotNull => Some(col.is_not_null()),
        DateCondition::Unspecified => None,
    }
}

pub fn type_condition_to_expr(col: Expr, cond: TypeCondition, val: i64) -> Option<SimpleExpr> {
    match cond {
        TypeCondition::Is => Some(col.eq(val)),
        TypeCondition::IsNot => Some(col.ne(val)),
        TypeCondition::Unspecified => None,
    }
}
pub fn type_string_condition_to_expr(
    col: Expr,
    cond: TypeCondition,
    val: &str,
) -> Option<SimpleExpr> {
    match cond {
        TypeCondition::Is => Some(col.eq(val)),
        TypeCondition::IsNot => Some(col.ne(val)),
        TypeCondition::Unspecified => None,
    }
}
