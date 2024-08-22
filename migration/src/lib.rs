pub use sea_orm_migration::prelude::*;

mod initialize_table;
mod migration_many_admin;
mod migration_signing_account;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(initialize_table::Migration),
            Box::new(migration_many_admin::Migration),
            Box::new(migration_signing_account::Migration),
        ]
    }
}
