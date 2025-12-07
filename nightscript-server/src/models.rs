use std::collections::BTreeMap;

use chrono::Utc;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Manifest {
    pub package: PackageSection,
    #[serde(default)]
    pub dependencies: BTreeMap<String, String>,
    #[serde(default)]
    pub registry: Option<RegistrySection>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PackageSection {
    pub name: String,
    pub version: String,
    pub language: String,
    pub description: Option<String>,
    pub license: Option<String>,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub min_runtime: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RegistrySection {
    pub url: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub username: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub created_at: String,
}

#[derive(Deserialize, Serialize)]
pub struct PackageSummary {
    pub name: String,
    pub description: Option<String>,
    pub latest_version: Option<String>,
}

#[derive(Serialize)]
pub struct PackageDetail {
    pub name: String,
    pub description: Option<String>,
    pub owner: String,
    pub versions: Vec<PackageVersion>,
}

#[derive(Serialize)]
pub struct PackageVersion {
    pub version: String,
    pub checksum: String,
    pub created_at: String,
}

#[derive(Serialize)]
pub struct PublishResponse {
    pub name: String,
    pub version: String,
}

#[derive(sqlx::FromRow)]
pub struct PackageRow {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: i64,
    pub owner_name: String,
}

#[derive(sqlx::FromRow)]
pub struct VersionRow {
    pub id: i64,
    pub version: String,
    pub checksum: String,
    pub created_at: String,
}

pub fn now_string() -> String {
    Utc::now().to_rfc3339()
}

pub fn parse_semver(input: &str) -> Result<Version, semver::Error> {
    Version::parse(input)
}

pub fn parse_req(input: &str) -> Result<VersionReq, semver::Error> {
    VersionReq::parse(input)
}
