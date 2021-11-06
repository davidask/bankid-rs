use std::{fmt::Display, net::IpAddr};

use serde::Deserialize;

use crate::PersonalNumer;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    pub order_ref: String,
    pub auto_start_token: String,
    pub qr_start_token: String,
    pub qr_start_secret: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    AlreadyInProgress,
    InvalidParameters,
    Canceled,
    Unauthorized,
    NotFound,
    RequestTimeout,
    UnsupportedMediaType,
    InternalError,
    Maintenance,
}

impl Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ClientError {
    pub error_code: ErrorCode,
    pub details: String,
}

impl Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error: {}: {}", self.error_code, self.details)
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum CollectHintCode {
    OutstandingTransaction,
    NoClient,
    Started,
    UserSign,
    ExpiredTransaction,
    CertificateErr,
    UserCancel,
    #[serde(rename = "cancelled")]
    Canceled,
    StartFailed,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub enum CollectStatus {
    Pending,
    Failed,
    Complete,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub personal_number: PersonalNumer,
    pub name: String,
    pub given_name: String,
    pub surname: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub ip_address: IpAddr,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Cert {
    pub not_berofe: String,
    pub not_after: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CompletionData {
    pub user: User,
    pub device: Device,
    pub cert: Cert,
    pub signature: String,
    pub ocsp_response: String,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "status")]
#[serde(rename_all = "camelCase")]
pub enum CollectResponse {
    #[serde(rename_all = "camelCase")]
    Pending { hint_code: CollectHintCode },
    #[serde(rename_all = "camelCase")]
    Failed { hint_code: CollectHintCode },
    #[serde(rename_all = "camelCase")]
    Complete { completion_data: CompletionData },
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CancelResponse {}
