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
    #[serde(default)]
    pub targets: TargetsSection,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PackageSection {
    pub name: String,
    pub version: String,
    #[serde(default = "default_language")]
    pub language: String,
    pub description: Option<String>,
    pub license: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    #[serde(default = "default_readme")]
    pub readme: String,
    #[serde(default = "default_min_runtime")]
    pub min_runtime: String,
    #[serde(default)]
    pub authors: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct TargetsSection {
    pub afml: Option<AfmlTarget>,
    pub rust: Option<RustTarget>,
    pub java: Option<JavaTarget>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AfmlTarget {
    #[serde(default = "default_afml_entry")]
    pub entry: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RustTarget {
    #[serde(rename = "crate")]
    pub crate_name: Option<String>,
    #[serde(default = "default_rust_lib_path")]
    pub lib_path: String,
    #[serde(default = "default_rust_build")]
    pub build: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JavaTarget {
    #[serde(default = "default_java_gradle_path")]
    pub gradle_path: String,
    pub group: Option<String>,
    pub artifact: Option<String>,
    pub version: Option<String>,
    #[serde(default = "default_java_build")]
    pub build: String,
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
    pub targets: TargetsSection,
}

#[derive(Serialize)]
pub struct PackageVersion {
    pub version: String,
    pub checksum: String,
    pub created_at: String,
    pub targets: TargetsSection,
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
    pub metadata_json: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct VersionRow {
    pub id: i64,
    pub version: String,
    pub checksum: String,
    pub created_at: String,
    pub targets_json: Option<String>,
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

fn default_language() -> String {
    "afml".to_string()
}

fn default_readme() -> String {
    "README.md".to_string()
}

fn default_min_runtime() -> String {
    ">=1.0.0".to_string()
}

fn default_afml_entry() -> String {
    "src/lib.afml".to_string()
}

fn default_rust_lib_path() -> String {
    "Cargo.toml".to_string()
}

fn default_rust_build() -> String {
    "cargo build --release".to_string()
}

fn default_java_gradle_path() -> String {
    "build.gradle".to_string()
}

fn default_java_build() -> String {
    "./gradlew jar".to_string()
}
