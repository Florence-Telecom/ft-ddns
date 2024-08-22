use std::fmt;
use std::net::Ipv4Addr;
use std::str::FromStr;

use rocket::request::FromRequest;
use rocket::*;

pub struct IP(Ipv4Addr);

impl IP {
    pub fn get(&self) -> Ipv4Addr {
        self.0
    }
}

impl fmt::Display for IP {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl FromStr for IP {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ipv4 = s.parse::<Ipv4Addr>().map_err(|_| ())?;
        Ok(Self(ipv4))
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for IP {
    type Error = ();
    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, ()> {
        match request.client_ip() {
            None => request::Outcome::Error((http::Status::BadRequest, ())),
            Some(ip) => match ip {
                std::net::IpAddr::V4(v4) => request::Outcome::Success(Self(v4)),
                std::net::IpAddr::V6(_) => {
                    request::Outcome::Error((http::Status::NotAcceptable, ()))
                }
            },
        }
    }
}
