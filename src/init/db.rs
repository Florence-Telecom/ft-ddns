use migration::{Migrator, MigratorTrait};
use rocket::fairing::AdHoc;
use sea_orm::ActiveModelTrait;
use sea_orm::ConnectOptions;
use sea_orm::Database;
use sea_orm::DbConn;
use sea_orm::DbErr;
use sea_orm::Set;
use std::env;

use crate::account::AdminAccountActiveModel;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

pub fn stage() -> AdHoc {
    AdHoc::on_ignite("Mounting database", |rocket| {
        Box::pin(async move {
            let db_uri = env::var("DATABASE_URI")
                .expect("Couldn't get the uri for the database from environment variables.");
            let db_opt = ConnectOptions::new(db_uri);

            let db = Database::connect(db_opt)
                .await
                .expect("Failed to connect to the database server.");

            if !env::var("SKIP_MIGRATION").is_ok_and(|v| !v.is_empty()) {
                log::warn!("Executing migration");
                Migrator::up(&db, None)
                    .await
                    .expect("Failed to migrate database.");

                log::warn!(
                    "{:?}",
                    Migrator::status(&db)
                        .await
                        .expect("Couldn't verify status of migration")
                );
            } else {
                log::warn!("Skipping migration")
            }

            if let Ok(password) = env::var("DDNS_ADMIN_PASSWORD") {
                bootstrap_admin_user(&db, &password).await;
            } else {
                log::warn!("No admin account was created, you will need to create your account manually in the database, or restart the program with the DDNS_ADMIN_PASSWORD environment variable set.");
            }

            rocket.manage(db)
        })
    })
}

async fn bootstrap_admin_user(db: &DbConn, admin_user_password: &str) {
    log::info!("Bootstrapping admin user into the database.");
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash: String = argon2
        .hash_password(admin_user_password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    let user = AdminAccountActiveModel {
        user: Set(String::from("admin")),
        password_hash: Set(password_hash),
    };

    match user.insert(db).await {
        Ok(_) => log::warn!("Bootstrapped admin user"),
        Err(error) => match error {
            DbErr::RecordNotInserted => log::warn!("Admin account already exists"),
            _ => {
                log::error!("An error prevented adding the admin user: {error}")
            }
        },
    }
}
