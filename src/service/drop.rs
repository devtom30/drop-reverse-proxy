use derive_new::new;
use serde::Deserialize;
use crate::repository::drop::DropRepo;
use crate::service::DropServiceT;

#[derive(Debug)]
pub enum ImportError {
    InvalidFileExtension,
    InvalidUnixEpoch,
    NoFileParentDirectory,
    InvalidParentDirectory,
    CantCreateDropUntarDirectory,
    CantCopyToUntarDirectory,
    CantOpenDropFile,
    CantUnpackDropFile,
    CantReadUntarDirectory,
    NoDropDescriptionFileFound,
    MissingTrackInDropArchive
}

#[derive(Clone, Deserialize, )]
pub struct DropRequest {
    artist_id: Option<String>,
    artist_name: Option<String>,
    playlist_name: String,
    tracks: Vec<String>
}

impl DropRequest {
    pub fn artist_id(&self) -> &Option<String> {
        &self.artist_id
    }

    pub fn artist_name(&self) -> &Option<String> {
        &self.artist_name
    }

    pub fn playlist_name(&self) -> &str {
        &self.playlist_name
    }

    pub fn tracks(&self) -> &Vec<String> {
        &self.tracks
    }
}

#[derive(Clone, Debug, new)]
pub struct DropService {
    drop_repository: Option<DropRepo>
}

impl DropServiceT for DropService {
    fn create_drop(&self, drop_request: DropRequest) -> Result<(), ImportError> {
        Ok(())
    }
}

impl DropService {
    pub fn drop_repository(&self) -> &Option<DropRepo> {
        &self.drop_repository
    }
}