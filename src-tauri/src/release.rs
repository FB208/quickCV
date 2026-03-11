pub const RELEASE_PAGE_URL: &str = "https://github.com/FB208/quickCV/releases";

pub fn normalize_version(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string()
}

pub fn release_tag_url(version: &str) -> String {
    format!("{RELEASE_PAGE_URL}/tag/v{}", normalize_version(version))
}
