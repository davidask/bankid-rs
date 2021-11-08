#![deny(missing_debug_implementations)]
#![deny(clippy::all)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(test, deny(warnings))]

use core::fmt;
use std::error::Error as StdError;
use std::fmt::{Debug, Display};
use std::str::FromStr;

use regex::{Match, Regex};
use reqwest::{self, Certificate, Identity as ReqwestIdentity, Url};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub mod request;
pub mod response;

pub type Identity = ReqwestIdentity;

#[derive(Debug)]
pub enum Error {
    InvalidPersonalNumber(&'static str),
    ReqwestError(reqwest::Error),
    ClientError(response::ClientError),
}

impl StdError for Error {}

impl From<reqwest::Error> for Error {
    fn from(inner: reqwest::Error) -> Self {
        Self::ReqwestError(inner)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPersonalNumber(reason) => write!(f, "Invalid personal number {}", reason),
            Self::ReqwestError(err) => write!(f, "Request failed: {}", err),
            Self::ClientError(err) => write!(f, "Client error: {}", err),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PersonalNumber {
    year: u16,
    month: u8,
    day: u8,
    last_four_digits: u16,
}

impl Serialize for PersonalNumber {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PersonalNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).and_then(|v| match PersonalNumber::from_str(v.as_str()) {
            Ok(personal_number) => Ok(personal_number),
            Err(error) => Err(error).map_err(serde::de::Error::custom),
        })
    }
}

impl PersonalNumber {
    #[inline]
    pub fn parse(s: &str) -> Result<Self, Error> {
        let re = Regex::new(r"^([19|20][0-9]{2}|[0-9]{4})([0-9]{2})([0-9]{2})[- ]?([0-9]{4})$")
            .expect("Invalid regular expression for PersonalNumber");

        if !re.is_match(s) {
            return Err(Error::InvalidPersonalNumber(
                "Personal number does not match expression",
            ));
        }

        if let Some(captures) = re.captures(s) {
            if captures.len() != 5 {
                return Err(Error::InvalidPersonalNumber(
                    "Unexpected capture length for RegEx matching personal number",
                ));
            }

            fn parse_part<F: FromStr>(m: Option<Match<'_>>) -> Result<F, Error> {
                match m {
                    Some(m) => match m.as_str().parse::<F>() {
                        Ok(val) => Ok(val),
                        Err(_) => Err(Error::InvalidPersonalNumber(
                            "Failed to parse match to numeric value",
                        )),
                    },
                    None => Err(Error::InvalidPersonalNumber(
                        "Expected match not found for part of personal number",
                    )),
                }
            }

            return Ok(PersonalNumber {
                year: parse_part(captures.get(1))?,
                month: parse_part(captures.get(2))?,
                day: parse_part(captures.get(3))?,
                last_four_digits: parse_part(captures.get(4))?,
            });
        } else {
            Err(Error::InvalidPersonalNumber(
                "No captures matching personal number",
            ))
        }
    }
}

impl FromStr for PersonalNumber {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Display for PersonalNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}{:02}{:02}{:04}",
            self.year, self.month, self.day, self.last_four_digits
        )
    }
}

#[cfg(feature = "rocket")]
use rocket::request::FromParam;

#[cfg(feature = "rocket")]
impl<'a> FromParam<'a> for PersonalNumber {
    type Error = Error;

    fn from_param(param: &'a str) -> Result<Self, Error> {
        Ok(PersonalNumber::parse(param)?)
    }
}

#[derive(Debug)]
pub enum Endpoint {
    Test,
    Production(Identity),
}

impl Endpoint {
    fn create_ca_root(&self) -> Certificate {
        Certificate::from_pem(match self {
            Self::Test => include_bytes!("./cert/ca-test.pem"),
            Self::Production(_) => include_bytes!("./cert/ca-prod.pem"),
        })
        .expect("Failed to create ca root certificate")
    }

    fn create_client(&self) -> reqwest::Client {
        let identity: Identity = match &self {
            Self::Test => Identity::from_pkcs12_der(
                include_bytes!("cert/FPTestcert3_20200618.p12"),
                "qwerty123",
            )
            .expect("Failed to create test identity"),
            Self::Production(identity) => identity.to_owned(),
        };

        reqwest::Client::builder()
            .add_root_certificate(self.create_ca_root())
            .identity(identity)
            .build()
            .expect("Failed to create HTTP client")
    }

    fn url(&self, path: &str) -> Url {
        match &self {
            Self::Test => Url::parse("https://appapi2.test.bankid.com/rp/v5.1/")
                .expect("Invalid BaseURL for test endpoint"),
            Self::Production(_) => Url::parse("https://appapi2.bankid.com/rp/v5.1/")
                .expect("Invalid BaseURL for production endpoint"),
        }
        .join(path)
        .expect("Failed to append path to base url")
    }
}

#[derive(Debug)]
pub struct Client {
    reqwest_client: reqwest::Client,
    endpoint: Endpoint,
}

impl Client {
    #[inline]
    pub fn new(endpoint: Endpoint) -> Client {
        Client {
            reqwest_client: endpoint.create_client(),
            endpoint,
        }
    }

    pub async fn auth(
        &self,
        request: request::AuthRequest,
    ) -> Result<response::OrderResponse, Error> {
        let request = self
            .reqwest_client
            .post(self.endpoint.url("auth"))
            .json(&request)
            .build()?;

        Ok(self.send(request).await?)
    }

    pub async fn collect(
        &self,
        request: request::CollectRequest,
    ) -> Result<response::CollectResponse, Error> {
        let request = self
            .reqwest_client
            .post(self.endpoint.url("collect"))
            .json(&request)
            .build()?;

        Ok(self.send(request).await?)
    }

    pub async fn sign(
        &self,
        request: request::SignRequest,
    ) -> Result<response::OrderResponse, Error> {
        let request = self
            .reqwest_client
            .post(self.endpoint.url("sign"))
            .json(&request)
            .build()?;

        Ok(self.send(request).await?)
    }

    pub async fn cancel(&self, request: request::CancelRequest) -> Result<(), Error> {
        let request = self
            .reqwest_client
            .post(self.endpoint.url("cancel"))
            .json(&request)
            .build()?;

        Ok(self
            .send::<response::CancelResponse>(request)
            .await
            .map(|_| ())?)
    }

    async fn send<T>(&self, request: reqwest::Request) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        let response = self.reqwest_client.execute(request).await?;

        if response.status().is_success() {
            Ok(response.json::<T>().await?)
        } else {
            let err = response.json::<response::ClientError>().await?;
            Err(Error::ClientError(err))
        }
    }
}

#[cfg(doctest)]
#[macro_use]
extern crate doc_comment;

#[cfg(doctest)]
doctest!("../README.md");

#[cfg(test)]
mod tests {
    use std::{
        net::{IpAddr, Ipv4Addr},
        str::FromStr,
    };

    use crate::{request, Client, Endpoint, PersonalNumber};

    #[test]
    fn test_pno_to_string() {
        let result = PersonalNumber {
            year: 1999,
            month: 1,
            day: 3,
            last_four_digits: 0101,
        };
        assert_eq!(result.to_string(), "199901030101");
    }

    #[test]
    fn test_pno_parse() {
        let result = PersonalNumber::from_str("198710101234").expect("Parsing failed");
        assert_eq!(result.year, 1987);
        assert_eq!(result.month, 10);
        assert_eq!(result.day, 10);
        assert_eq!(result.last_four_digits, 1234);
    }

    #[test]
    fn test_pno_serde() {
        fn case(year: u16, month: u8, day: u8, lfd: u16) {
            let raw = format!(r#""{:04}{:02}{:02}{:04}""#, year, month, day, lfd);

            let pno: PersonalNumber =
                serde_json::from_str(raw.as_str()).expect("Failed to deserialize pno");

            assert_eq!(pno.year, year);
            assert_eq!(pno.month, month);
            assert_eq!(pno.day, day);
            assert_eq!(pno.last_four_digits, lfd);

            assert_eq!(
                serde_json::to_string(&pno).expect("Failed to serialize pno"),
                raw
            );
        }

        for n in 1900u16..2000 {
            case(
                n,
                u8::try_from(n % 12).expect("Invalid convert"),
                u8::try_from(n % 30).expect("Invalid convert"),
                n,
            );
        }
    }

    #[tokio::test]
    async fn test_integration() {
        let client = Client::new(Endpoint::Test);

        let auth_response = client
            .auth(request::AuthRequest {
                end_user_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
                personal_number: PersonalNumber {
                    year: 1987,
                    month: 10,
                    day: 10,
                    last_four_digits: 0101,
                },
                requirement: None,
            })
            .await
            .expect("Auth request failed");

        client
            .collect(request::CollectRequest {
                order_ref: auth_response.order_ref.to_owned(),
            })
            .await
            .expect("Collect request failed");

        client
            .cancel(request::CancelRequest {
                order_ref: auth_response.order_ref.to_owned(),
            })
            .await
            .expect("Cancel request failed");
    }
}
