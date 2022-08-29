use std::fmt::{Display, Formatter};
use std::time::SystemTime;
use hyper::{Body, body};
use serde::{Deserialize, Serialize};
use eyre::Result;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invitee {
    pub id: i32,
    pub first_name: String,
    pub rsvp: Option<(RsvpDetails, SystemTime)>
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ClientRSVP {
    pub first_name: String,
    pub details: RsvpDetails
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct RsvpDetails {
    pub phone_number: Option<i64>,
    pub email_address: Option<String>
}

impl Display for RsvpDetails {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match (self.phone_number, Some("")) {
            (None, None) => write!(f, "No contact info"),
            (Some(phone_no), None) => write!(f, "Phone number: {}", phone_no),
            (None, Some(email)) => write!(f, "Email address: {}", email),
            (Some(phone_no), Some(email)) => write!(f, "Phone number: {}\n Email address: {}", phone_no, email)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostPath {
    EnterRsvp
}

impl PostPath {
    pub fn from_str(path: &str) -> Option<Self> {
        if path == "enter-rsvp" {
            Some(PostPath::EnterRsvp)
        } else {
            None
        }
    }
}

impl AsRef<str> for PostPath {
    fn as_ref(&self) -> &str {
        match self {
            &PostPath::EnterRsvp => "enter-rsvp"
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub enum ServerResponse {
    Success,
    NotInvited,
    AlreadyRSVPed(u64)
}

macro_rules! encode_decode_as_http_body {
    ($typename:ident) => {
        impl $typename {
            pub fn encode(self) -> Result<Body> {
                let string = serde_json::to_string(&self)?;
                Ok(Body::from(string))
            }

            pub async fn decode(body: Body) -> Result<Self> {
                let bytes = body::to_bytes(body).await?;
                let string = std::str::from_utf8(&bytes)?;
                Ok(serde_json::from_str(string)?)
            }
        }
    }
}

encode_decode_as_http_body!(ClientRSVP);
encode_decode_as_http_body!(ServerResponse);
