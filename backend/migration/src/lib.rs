pub use sea_orm_migration::prelude::*;

mod cache;
mod m20220101_000001_create_table;
mod m20220101_000002_create_rel_table;
mod m20250418_145233_create_table;

pub struct Migrator;
pub struct MigratorCacheDB;

#[macro_export]
macro_rules! unique_index_name {
    ($entity:path, $($column:path),+) => {
        concat!(
            stringify!($entity), "_",
            $(stringify!($column), "_"),+
        ).trim_end_matches('_')
    };
}

#[macro_export]
macro_rules! unique_index {
    ($entity:path, $($column:path),+) => {
        sea_query::Index::create()
            .name(unique_index_name!($entity, $($column),+))
            .table($entity)
            $(.col($column))+
            .unique()
            .to_owned()
    };
}

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20220101_000002_create_rel_table::Migration),
            Box::new(m20250418_145233_create_table::Migration),
        ]
    }
}
