use sea_orm_migration::prelude::*;

/// First migration
///
/// Creates the account table
/// Creates the admin_account table
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(Account::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Account::Domain)
                            .string_len(255)
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Account::PasswordHash).string().not_null())
                    .col(ColumnDef::new(Account::Disabled).boolean().default(false))
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(AdminAccount::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(AdminAccount::User)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(AdminAccount::PasswordHash)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Account::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(AdminAccount::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Account {
    Table,
    Domain,
    PasswordHash,
    Disabled,
}

#[derive(DeriveIden)]
enum AdminAccount {
    Table,
    User,
    PasswordHash,
}
