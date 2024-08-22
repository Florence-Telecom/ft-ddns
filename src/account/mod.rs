mod admin_account;
mod password_account;
mod signing_account;

use sea_orm::{DbConn, DbErr};

pub use admin_account::{ActiveModel as AdminAccountActiveModel, AdminAccount};
pub use password_account::PasswordAccount;
pub use signing_account::{PublicKey, SigningAccount};

pub trait Account {
    fn get_domain(&self) -> &str;
}

pub async fn exists(domain: &str, db: &DbConn) -> Result<bool, DbErr> {
    if !PasswordAccount::exists(domain, db).await? {
        return SigningAccount::exists(domain, db).await;
    }

    Ok(true)
}
