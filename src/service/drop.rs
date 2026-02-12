use crate::repository::artist::Artist;
use crate::repository::drop::Drop;
use crate::repository::playlist::Playlist;
use crate::repository::{Repo, RepoByName};
use crate::service::DropServiceT;
use async_trait::async_trait;
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

#[derive(Debug, Deserialize,)]
pub struct DropService<T, U, V>
where
    T: Repo<Drop> + Send + Sync,
    U: Repo<Artist> + Send + Sync,
    V: Repo<Playlist> + Send + Sync,
{
    drop_repository: T,
    artist_repository: U,
    playlist_repository: V,
}

impl<T, U, V> Clone for DropService<T, U, V>
where
    T: Repo<Drop> + Send + Sync + Clone,
    U: Repo<Artist> + Send + Sync + Clone,
    V: Repo<Playlist> + Send + Sync + Clone, {
    fn clone(&self) -> Self {
        DropService::new(
            self.drop_repository.clone(),
            self.artist_repository.clone(),
            self.playlist_repository.clone()
        )
    }
}

impl<T, U, V> DropService<T, U, V>
where
    T: Repo<Drop> + Send + Sync,
    U: Repo<Artist> + Send + Sync,
    V: Repo<Playlist> + Send + Sync,
{
    pub fn new(
        drop_repository: T,
        artist_repository: U,
        playlist_repository: V,
    ) -> DropService<T, U, V>
    where
        T: Sized,
        U: Sized,
        V: Sized,
    {
        /*if drop_repository.drop() {
            println!("drop repository not set, can't create drop");
            return Err(ImportError::DropRepositoryIsNone)
        }
        if artist_repository.is_none() {
            println!("artist repository not set, can't create drop");
            return Err(ImportError::ArtistRepositoryIsNone)
        }
        if playlist_repository.is_none() {
            println!("playlist repository not set, can't create drop");
            return Err(ImportError::PlaylistRepositoryIsNone)
        }*/

        Self {
            drop_repository,
            artist_repository,
            playlist_repository,
        }
    }

    pub fn drop_repository(&self) -> &T {
        &self.drop_repository
    }

    pub fn artist_repository(&self) -> &U {
        &self.artist_repository
    }

    pub fn playlist_repository(&self) -> &V {
        &self.playlist_repository
    }
}

#[async_trait]
impl<T, U, V> DropServiceT for DropService<T, U, V>
where
    T: Repo<Drop> + Send + Sync,
    U: Repo<Artist> + RepoByName<Artist> + Send + Sync,
    V: Repo<Playlist> + Send + Sync,
{
    async fn create_drop(
        &self,
        drop_import_path: String,
        drop_request: DropRequest,
    ) -> Result<(), ImportError> {

        // artist_id XOR artist_name
        if drop_request.artist_id.is_some() && drop_request.artist_name.is_some() {
            return Err(ImportError::ArtistIdAndArtistNameAreBothPresent)
        }
        let mut drop_artist_id = 0;
        // artist_id exists
        if let Some(artist_id) = drop_request.artist_id {
            // check artist_id exists
            drop_artist_id = self.artist_repository.get(artist_id)
                .await
                .or(Err(ImportError::InvalidArtistId))?.id();
        } else if let Some(artist_name) = drop_request.artist_name {
            // check artist_name exists
            drop_artist_id = self.artist_repository.get_by_name(&artist_name)
                .await
                .or(Err(ImportError::CantCreateArtistFromArtistName))?.id();
        }

        // create playlist
        let playlist_id = self.playlist_repository
            .save_or_update(&Playlist::new(0, drop_request.playlist_name))
            .await
            .or(Err(ImportError::CantCreatePlaylistFromPlaylistName))?;

        // create drop
        self.drop_repository
            .save_or_update(&Drop::new(0, drop_artist_id, 0, playlist_id))
            .await
            .or(Err(ImportError::CantCreateDropFromDropRequest))?;

        //TODO moving the files

        Ok(())
    }
}