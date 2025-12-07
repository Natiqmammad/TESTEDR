use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub signed_in: Option<String>,
}

#[derive(Clone)]
pub struct PackageListItem {
    pub name: String,
    pub description: Option<String>,
    pub owner: String,
    pub latest_version: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Clone)]
pub struct PaginationContext {
    pub page: usize,
    pub total_pages: usize,
    pub has_prev: bool,
    pub has_next: bool,
}

#[derive(Template)]
#[template(path = "packages.html")]
pub struct PackagesTemplate {
    pub signed_in: Option<String>,
    pub packages: Vec<PackageListItem>,
    pub pagination: PaginationContext,
    pub search_value: String,
    pub search_param: Option<String>,
    pub sort: String,
}

#[derive(Clone)]
pub struct PackageVersionView {
    pub version: String,
    pub checksum: String,
    pub created_at: String,
}

#[derive(Clone)]
pub struct PackageDetailView {
    pub name: String,
    pub description: Option<String>,
    pub owner: String,
    pub latest_version: Option<String>,
    pub install_add: String,
    pub install_install: String,
}

#[derive(Template)]
#[template(path = "package.html")]
pub struct PackageTemplate {
    pub signed_in: Option<String>,
    pub package: PackageDetailView,
    pub versions: Vec<PackageVersionView>,
    pub readme_html: Option<String>,
}

#[derive(Clone)]
pub struct OwnerPackageView {
    pub name: String,
    pub description: Option<String>,
    pub latest_version: Option<String>,
}

#[derive(Template)]
#[template(path = "owner.html")]
pub struct OwnerTemplate {
    pub signed_in: Option<String>,
    pub owner_name: String,
    pub packages: Vec<OwnerPackageView>,
}

#[derive(Template)]
#[template(path = "login.html")]
pub struct LoginTemplate {
    pub signed_in: Option<String>,
    pub token: Option<String>,
    pub error: Option<String>,
}

#[derive(Template)]
#[template(path = "error.html")]
pub struct ErrorTemplate {
    pub signed_in: Option<String>,
    pub title: String,
    pub message: String,
}
