#[derive(serde::Serialize)]
pub(crate) struct IdentityView {
    pub(crate) key: String,
    pub(crate) value: String,
}
