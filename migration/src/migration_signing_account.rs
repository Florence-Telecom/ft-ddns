use sea_orm_migration::prelude::*;

/// Third migration
///
/// Renames the account table to the password_account table
/// Creates the signing_account table
#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .rename_table(
                Table::rename()
                    .table(Account::Table, PasswordAccount::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(SigningAccount::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(SigningAccount::Domain).string().not_null())
                    .col(ColumnDef::new(SigningAccount::PublicKey).text().not_null())
                    .col(
                        ColumnDef::new(SigningAccount::CreatedBy)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(SigningAccount::Disabled)
                            .boolean()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .rename_table(
                Table::rename()
                    .table(PasswordAccount::Table, Account::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_table(Table::drop().table(SigningAccount::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Account {
    Table,
}

#[derive(DeriveIden)]
enum PasswordAccount {
    Table,
}

#[derive(DeriveIden)]
enum SigningAccount {
    Table,
    Domain,
    CreatedBy,
    PublicKey,
    Disabled,
}
