use crate::{utils::compare_with_hash, utils::Credentials};
use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
use rand::rngs::OsRng;
use rocket::{
    async_trait, http,
    request::{self, FromRequest},
    Request,
};
use rocket_basicauth::BasicAuth;
use sea_orm::{entity::prelude::*, Set};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "admin_account")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user: String,
    pub password_hash: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

pub struct AdminAccount(String);

impl AdminAccount {
    #[allow(unused)]
    pub fn get_user(&self) -> &str {
        &self.0
    }
}

impl From<crate::utils::Credentials> for ActiveModel {
    fn from(value: Credentials) -> Self {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash: String = argon2
            .hash_password(value.password.as_bytes(), &salt)
            .unwrap()
            .to_string();

        Self {
            user: Set(value.username),
            password_hash: Set(password_hash),
        }
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for AdminAccount {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, ()> {
        let db: &DbConn = request.rocket().state::<DbConn>().unwrap();
        let auth: BasicAuth;
        match request.guard::<BasicAuth>().await {
            request::Outcome::Error(_) => {
                return request::Outcome::Error((http::Status::BadRequest, ()))
            }
            request::Outcome::Forward(_) => {
                return request::Outcome::Error((http::Status::BadRequest, ()))
            }
            request::Outcome::Success(a) => auth = a,
        }

        let admin_account: Option<Model> = Entity::find()
            .filter(Column::User.eq(&auth.username))
            .one(db)
            .await
            .unwrap();

        match admin_account {
            None => request::Outcome::Error((http::Status::Unauthorized, ())),
            Some(d) => match compare_with_hash(&auth.password, &d.password_hash) {
                Err(_) => request::Outcome::Error((http::Status::Unauthorized, ())),
                Ok(()) => request::Outcome::Success(AdminAccount(d.user)),
            },
        }
    }
}
