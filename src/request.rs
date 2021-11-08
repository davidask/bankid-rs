use serde::Serialize;
use std::net::IpAddr;
use uuid::Uuid;

use crate::PersonalNumber;

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Requirement {
    #[serde(skip_serializing_if = "Option::is_none")]
    certificate_policies: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    allow_fingerprint: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    auto_start_token_required: Option<bool>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthRequest {
    pub end_user_ip: IpAddr,

    pub personal_number: PersonalNumber,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<Requirement>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SignRequest {
    pub end_user_ip: IpAddr,

    pub personal_number: PersonalNumber,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub requirement: Option<Requirement>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_visible_data: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_non_visible_data: Option<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CollectRequest {
    pub order_ref: Uuid,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CancelRequest {
    pub order_ref: Uuid,
}
