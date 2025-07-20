use crate::msg::{
    self,
    filter::{AlbumFilter, IntCondition, StringCondition},
};
use qcm_core::model as sqlm;
use sea_orm::sea_query::{Expr, SimpleExpr};
use sea_orm::{QueryFilter, RelationTrait};

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
                            .to_owned();
                        Expr::exists(subquery)
                    }),
                Some(Payload::TitleFilter(title)) => {
                    title.get_expr_from_col(sqlm::album::Column::Name)
                }
                Some(Payload::TrackFilter(track)) => {
                    track.get_expr_from_col(sqlm::album::Column::TrackCount)
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
impl_string_filter!(msg::filter::NameFilter);
impl_string_filter!(msg::filter::TitleFilter);
impl_string_filter!(msg::filter::ArtistNameFilter);
impl_id_filter!(msg::filter::AlbumArtistIdFilter);

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
