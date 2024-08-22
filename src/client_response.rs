#![allow(unused)]
use rocket::*;

#[derive(Responder)]
pub enum ClientResponse {
    #[response(status = 200)]
    Ok(String),

    #[response(status = 400)]
    BadRequest(String),

    #[response(status = 401)]
    Unauthorized(String),

    #[response(status = 406)]
    NotAcceptable(String),

    #[response(status = 409)]
    Conflict(String),

    #[response(status = 500)]
    InternalServerError(String),

    #[response(status = 501)]
    NotImplemented(String),

    #[response(status = 503)]
    ServiceUnavailable(String),
}
