use crate::service::drop::DropRequest;

pub mod drop;

pub enum ServiceType {
    DropService(dyn DropServiceT)
}

pub trait DropServiceT {
    fn create_drop(&self, drop_request: DropRequest) -> Result<(), drop::ImportError>;
}
