use crate::repository::artist::ArtistRepo;
use crate::repository::drop::{Drop, DropRepo};
use crate::repository::playlist::{Playlist, PlaylistRepo};
use crate::repository::{Repo, RepoByName};
use crate::service::DropServiceT;
use derive_new::new;
use serde::Deserialize;

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
    MissingTrackInDropArchive,
    ArtistIdAndArtistNameAreBothPresent,
    InvalidArtistId,
    DropRepositoryIsNone,
    ArtistRepositoryIsNone,
    PlaylistRepositoryIsNone,
    CantCreateArtistFromArtistName,
    CantCreateDropFromDropRequest,
    CantCreatePlaylistFromPlaylistName
}

#[derive(Clone, Deserialize, )]
pub struct DropRequest {
    artist_id: Option<i32>,
    artist_name: Option<String>,
    playlist_name: String,
    tracks: Vec<String>
}

impl DropRequest {
    pub fn artist_id(&self) -> &Option<i32> {
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
    drop_repository: Option<DropRepo>,
    artist_repository: Option<ArtistRepo>,
    playlist_repository: Option<PlaylistRepo>,
}

impl DropServiceT for DropService {
    async fn create_drop(&self, drop_import_path: String, drop_request: DropRequest) -> Result<(), ImportError> {
        if self.drop_repository.is_none() {
            println!("drop repository not set, can't create drop");
            return Err(ImportError::DropRepositoryIsNone)
        }
        if self.artist_repository.is_none() {
            println!("artist repository not set, can't create drop");
            return Err(ImportError::ArtistRepositoryIsNone)
        }
        if self.playlist_repository.is_none() {
            println!("playlist repository not set, can't create drop");
            return Err(ImportError::PlaylistRepositoryIsNone)
        }

        // artist_id XOR artist_name
        if drop_request.artist_id.is_some() && drop_request.artist_name.is_some() {
            return Err(ImportError::ArtistIdAndArtistNameAreBothPresent)
        }
        let mut drop_artist_id = 0;
        // artist_id exists
        if let Some(artist_id) = drop_request.artist_id {
            // check artist_id exists
            drop_artist_id = self.artist_repository.as_ref().unwrap().get(artist_id)
                .await
                .or(Err(ImportError::InvalidArtistId))?.id();
        } else if let Some(artist_name) = drop_request.artist_name {
            // check artist_name exists
            drop_artist_id = self.artist_repository.as_ref().unwrap().get_by_name(&artist_name)
                .await
                .or(Err(ImportError::CantCreateArtistFromArtistName))?.id();
        }

        // create playlist
        let playlist_id = self.playlist_repository.as_ref().unwrap()
            .save_or_update(&Playlist::new(0, drop_request.playlist_name))
            .await
            .or(Err(ImportError::CantCreatePlaylistFromPlaylistName))?;

        // create drop
        self.drop_repository.as_ref().unwrap()
            .save_or_update(&Drop::new(0, drop_artist_id, 0, playlist_id))
            .await
            .or(Err(ImportError::CantCreateDropFromDropRequest))?;

        //TODO moving the files

        Ok(())
    }
}

impl DropService {
    pub fn drop_repository(&self) -> &Option<DropRepo> {
        &self.drop_repository
    }

    pub fn artist_repository(&self) -> &Option<ArtistRepo> {
        &self.artist_repository
    }

    pub fn playlist_repository(&self) -> &Option<PlaylistRepo> {
        &self.playlist_repository
    }
}