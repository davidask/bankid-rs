use crate::PersonalNumber;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, net::IpAddr};
use uuid::Uuid;

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderResponse {
    pub order_ref: Uuid,
    pub auto_start_token: Uuid,
    pub qr_start_token: Uuid,
    pub qr_start_secret: Uuid,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum CollectStatus {
    Pending,
    Failed,
    Complete,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub personal_number: PersonalNumber,
    pub name: String,
    pub given_name: String,
    pub surname: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub ip_address: IpAddr,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Cert {
    pub not_before: String,
    pub not_after: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompletionData {
    pub user: User,
    pub device: Device,
    pub cert: Cert,
    pub signature: String,
    pub ocsp_response: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "status")]
#[serde(rename_all = "camelCase")]
pub enum CollectResponse {
    #[serde(rename_all = "camelCase")]
    Pending { hint_code: CollectHintCode, order_ref: Uuid },
    #[serde(rename_all = "camelCase")]
    Failed { hint_code: CollectHintCode },
    #[serde(rename_all = "camelCase")]
    Complete { completion_data: CompletionData, order_ref: Uuid },
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CancelResponse {}
