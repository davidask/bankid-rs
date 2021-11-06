use core::fmt;
use std::error::Error as StdError;
use std::fmt::Display;
use std::str::FromStr;

use regex::{Match, Regex};
use reqwest::{self, Certificate, Identity, Url};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub mod request;
pub mod response;

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

pub struct PersonalNumer {
    year: u16,
    month: u8,
    day: u8,
    last_four_digits: u16,
}

impl Serialize for PersonalNumer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.to_string().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PersonalNumer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).and_then(|v| match PersonalNumer::parse(v.as_str()) {
            Ok(personal_number) => Ok(personal_number),
            Err(error) => Err(error).map_err(serde::de::Error::custom),
        })
    }
}

impl PersonalNumer {
    #[inline]
    pub fn parse(value: &str) -> Result<PersonalNumer, Error> {
        let re = Regex::new(r"^([19|20]?[0-9]{2}|[0-9]{4})([0-9]{2})([0-9]{2})[- ]?([0-9]{4})$")
            .expect("Invalid regular expression for PersonalNumber");

        if !re.is_match(value) {
            return Err(Error::InvalidPersonalNumber(
                "Personal number does not match expression",
            ));
        }

        if let Some(captures) = re.captures(value) {
            if captures.len() != 5 {
                return Err(Error::InvalidPersonalNumber(
                    "Unexpected capture length for RegEx matching personal number",
                ));
            }

            fn parse_part<'t, F: FromStr>(m: Option<Match<'t>>) -> Result<F, Error> {
                match m {
                    Some(m) => match m.as_str().parse::<F>() {
                        Ok(val) => Ok(val),
                        Err(_) => {
                            return Err(Error::InvalidPersonalNumber(
                                "Failed to parse match to numeric value",
                            ))
                        }
                    },
                    None => {
                        return Err(Error::InvalidPersonalNumber(
                            "Expected match not found for part of personal number",
                        ))
                    }
                }
            }

            return Ok(PersonalNumer {
                year: parse_part(captures.get(1))?,
                month: parse_part(captures.get(2))?,
                day: parse_part(captures.get(3))?,
                last_four_digits: parse_part(captures.get(4))?,
            });
        } else {
            return Err(Error::InvalidPersonalNumber(
                "No captures matching personal number",
            ));
        }
    }
}

impl Display for PersonalNumer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:04}{:02}{:02}{:04}",
            self.year, self.month, self.day, self.last_four_digits
        )
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};

    use reqwest::{Certificate, Identity};

    use crate::{request, Client, PersonalNumer};

    #[test]
    fn test_pno_to_string() {
        let result = PersonalNumer {
            year: 1999,
            month: 1,
            day: 3,
            last_four_digits: 0101,
        };
        assert_eq!(result.to_string(), "199901030101");
    }

    #[test]
    fn test_pno_parse() {
        let result = PersonalNumer::parse("198710101234").expect("Parsing failed");
        assert_eq!(result.year, 1987);
        assert_eq!(result.month, 10);
        assert_eq!(result.day, 10);
        assert_eq!(result.last_four_digits, 1234);
    }

    #[test]
    fn test_pno_serde() {
        fn case(year: u16, month: u8, day: u8, lfd: u16) {
            let raw = format!(r#""{:04}{:02}{:02}{:04}""#, year, month, day, lfd);
            println!("{}", raw);
            let pno: PersonalNumer =
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
    async fn test_auth() {
        let ca_root_certificate = base64::decode("MIIFvjCCA6agAwIBAgIITyTh/u1bExowDQYJKoZIhvcNAQENBQAwYjEkMCIGA1UECgwbRmluYW5zaWVsbCBJRC1UZWtuaWsgQklEIEFCMRowGAYDVQQLDBFJbmZyYXN0cnVjdHVyZSBDQTEeMBwGA1UEAwwVQmFua0lEIFNTTCBSb290IENBIHYxMB4XDTExMTIwNzEyMzQwN1oXDTM0MTIzMTEyMzQwN1owYjEkMCIGA1UECgwbRmluYW5zaWVsbCBJRC1UZWtuaWsgQklEIEFCMRowGAYDVQQLDBFJbmZyYXN0cnVjdHVyZSBDQTEeMBwGA1UEAwwVQmFua0lEIFNTTCBSb290IENBIHYxMIICIjANBgkqhkiG9w0BAQEFAAOCAg8AMIICCgKCAgEAwVA4snZiSFI3r64LvYu4mOsI42A9aLKEQGq4IZo257iqvPH82SMvgBJgE52kCx7gQMmZ7iSm39CEA19hlILh8JEJNTyJNxMxVDN6cfJP1jMHJeTES1TmVbWUqGyLpyT8LCJhC9Vq4W3t/O1svGJNOUQIQL4eAHSvWTVoalxzomJhOn97ENjXAt4BLb6sHfVBvmB5ReK0UfwpNACFM1RN8btEaDdWC4PfA72yzV3wK/cY5h2k1RM1s19PjoxnpJqrmn4qZmP4tN/nk2d7c4FErJAP0pnNsll1+JfkdMfiPD35+qcclpspzP2LpauQVyPbO21Nh+EPtr7+Iic2tkgz0g1kK0IL/foFrJ0Ievyr3Drm2uRnA0esZ45GOmZhE22mycEX9l7w9jrdsKtqs7N/T46hil4xBiGblXkqKNG6TvARk6XqOp3RtUvGGaKZnGllsgTvP38/nrSMlszNojrlbDnm16GGoRTQnwr8l+Yvbz/ev/e6wVFDjb52ZB0Z/KTfjXOl5cAJ7OCbODMWf8Na56OTlIkrk5NyU/uGzJFUQSvGdLHUipJ/sTZCbqNSZUwboI0oQNO/Ygez2J6zgWXGpDWiN4LGLDmBhB3T8CMQu9J/BcFvgjnUyhyim35kDpjVPC8nrSir5OkaYgGdYWdDuv1456lFNPNNQcdZdt5fcmMCAwEAAaN4MHYwHQYDVR0OBBYEFPgqsux5RtcrIhAVeuLBSgBuRDFVMA8GA1UdEwEB/wQFMAMBAf8wHwYDVR0jBBgwFoAU+Cqy7HlG1ysiEBV64sFKAG5EMVUwEwYDVR0gBAwwCjAIBgYqhXBOAQQwDgYDVR0PAQH/BAQDAgEGMA0GCSqGSIb3DQEBDQUAA4ICAQAJOjUOS2GJPNrrrqf539aN1/EbUj5ZVRjG4wzVtX5yVqPGcRZjUQlNTcfOpwPoczKBnNX2OMF+Qm94bb+xXc/08AERqJJ3FPKu8oDNeK+Rv1X4nh95J4RHZcvl4AGhECmGMyhyCea0qZBFBsBqQR7oC9afYOxsSovaPqX31QMLULWUYoBKWWHLVVIoHjAmGtAzMkLwe0/lrVyApr9iyXWhVr+qYGmFGw1+rwmvDmmSLWNWawYgH4NYxTf8z5hBiDOdAgilvyiAF8Yl0kCKUB2fAPhRNYlEcN+UP/KL24h/pB+hZ9mvR0tM6nW3HVZaDrvRz4VihZ8vRi3fYnOAkNE6kZdrrdO7LdBc9yYkfQdTcy0N+Aw7q4TkQ8npomrVmTKaPhtGhA7VICyRNBVcvyoxr+CY7aRQyHn/C7n/jRsQYxs7uc+msq6jRS4HPK8olnF9usWZX6KY+8mweJiTE4uN4ZUUBUtt8WcXXDiK/bxEG2amjPcZ/b4LXwGCJb+aNWP4+iY6kBKrMANs01pLvtVjUS9RtRrY3cNEOhmKhO0qJSDXhsTcVtpbDr37UTSqQVw83dReiARPwGdURmmkaheH6z4k6qEUSXuFch0w53UAc+1aBXR1bgyFqMdy7Yxib2AYu7wnrHioDWqP6DTkUSUeMB/zqWPM/qx6QNNOcaOcjA==")
        .expect("Failed to decode");

        let root_ca =
            Certificate::from_pem(&ca_root_certificate).expect("failed to create CA root");

        let certificate =
            reqwest::get("https://www.bankid.com/assets/bankid/rp/FPTestcert3_20200618.p12")
                .await
                .expect("Failed to load certificate for test")
                .bytes()
                .await
                .expect("Failed to get bytes from certificate for test");

        let identity =
            Identity::from_pkcs12_der(certificate.as_ref(), "qwerty123").expect("Identity");

        let client = Client::new(root_ca, identity, true).expect("CLIENT");

        let auth_response = client
            .auth(request::AuthRequest {
                end_user_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
                personal_number: PersonalNumer {
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
                order_ref: &auth_response.order_ref,
            })
            .await
            .expect("Collect request failed");

        client
            .cancel(request::CancelRequest {
                order_ref: &auth_response.order_ref,
            })
            .await
            .expect("Cancel request failed");
    }
}

pub struct Client {
    reqwest_client: reqwest::Client,
    pub use_test_environment: bool,
}

impl Client {
    #[inline]
    pub fn new(
        root_certificate: Certificate,
        identity: Identity,
        use_test_environment: bool,
    ) -> Result<Client, Error> {
        Ok(Client {
            reqwest_client: reqwest::Client::builder()
                .add_root_certificate(root_certificate)
                .identity(identity)
                .build()?,
            use_test_environment,
        })
    }

    pub async fn auth(
        &self,
        request: request::AuthRequest,
    ) -> Result<response::OrderResponse, Error> {
        let request = self
            .reqwest_client
            .post(self.base_uri("auth"))
            .json(&request)
            .build()?;

        Ok(self.send(request).await?)
    }

    pub async fn collect<'a>(
        &self,
        request: request::CollectRequest<'a>,
    ) -> Result<response::CollectResponse, Error> {
        let request = self
            .reqwest_client
            .post(self.base_uri("collect"))
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
            .post(self.base_uri("sign"))
            .json(&request)
            .build()?;

        Ok(self.send(request).await?)
    }

    pub async fn cancel<'a>(&self, request: request::CancelRequest<'a>) -> Result<(), Error> {
        let request = self
            .reqwest_client
            .post(self.base_uri("cancel"))
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

    fn base_uri(&self, path: &str) -> Url {
        let url_str = match self.use_test_environment {
            true => "https://appapi2.test.bankid.com/rp/v5.1/",
            false => "https://appapi2.bankid.com/rp/v5.1/",
        };

        let url = Url::parse(url_str).expect(format!("Invalid URL {}", url_str).as_str());

        url.join(path)
            .expect(format!("Failed to join path {} onto url {}", path, url).as_str())
    }
}
