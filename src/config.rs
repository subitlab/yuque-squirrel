use std::fmt::Display;

use serde::Deserialize;

use crate::Token;

#[derive(Debug, Deserialize)]
pub struct Config {
    /// The host URL of Yuque organization.
    pub host: String,
    /// Token of your account, or group.
    pub token: Token,
    /// The target user/group to backup.
    pub target: Target,
    /// Request limitation per second.
    pub limit: usize,
}

#[derive(Debug, Deserialize)]
pub struct Target {
    #[serde(rename = "type")]
    pub ty: TargetType,
    pub login: String,
}

#[derive(Debug, Deserialize)]
pub enum TargetType {
    #[serde(rename = "groups")]
    Group,
    #[serde(rename = "users")]
    User,
}

impl Display for TargetType {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetType::Group => write!(f, "groups"),
            TargetType::User => write!(f, "users"),
        }
    }
}
