use crate::service::drop::DropRequest;

pub mod drop;

pub trait DropService {
    fn create_drop(&self, drop_request: DropRequest) -> Result<(), drop::ImportError>;
}