use crate::service::drop::{DropRequest, DropService};

pub mod drop;

pub enum ServiceEnum {
    DropService(DropService)
}

pub trait DropServiceT {
    async fn create_drop(&self, drop_import_path: String, drop_request: DropRequest) -> Result<(), drop::ImportError>;
}
