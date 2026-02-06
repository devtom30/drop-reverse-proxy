use crate::service;
use crate::service::drop::{DropRequest, DropService};

pub mod drop;

pub enum ServiceEnum {
    DropService(DropService)
}

pub trait DropServiceT {
    fn create_drop(&self, drop_request: DropRequest) -> Result<(), drop::ImportError>;
}

impl Clone for ServiceEnum {
    fn clone(&self) -> Self {
        match self {
            ServiceEnum::DropService(drop_service) => ServiceEnum::DropService(DropService::new(drop_service.drop_repository().clone()))
        }
    }
}
