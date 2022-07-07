#![feature(let_chains)]
// mod nsp;
mod unpacked;
mod zarchive;

use self::{unpacked::Unpacked, zarchive::ZArchive};
use enum_dispatch::enum_dispatch;
use moka::sync::Cache;
use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use uk_content::{canonicalize, resource::ResourceData};

#[derive(Debug, thiserror::Error)]
pub enum ROMError {
    #[error("File not found in game dump: {0}\n(Using ROM at {1})")]
    FileNotFound(String, PathBuf),
    #[error("Missing required {0} folder in game dump\n(Using ROM at {1})")]
    MissingDumpDir(&'static str, PathBuf),
    #[error("Invalid resource path: {0}")]
    InvalidPath(String),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
    #[error(transparent)]
    WUAError(#[from] ::zarchive::ZArchiveError),
    #[error(transparent)]
    UKError(#[from] uk_content::UKError),
    #[error("{0}")]
    OtherMessage(&'static str),
}

impl From<ROMError> for uk_content::UKError {
    fn from(err: ROMError) -> Self {
        Self::Any(err.into())
    }
}

type ResourceCache = Cache<String, Arc<ResourceData>>;
pub type Result<T> = std::result::Result<T, ROMError>;

#[enum_dispatch(ROMSource)]
pub trait ROMReader {
    fn get_file_data(&self, name: impl AsRef<Path>) -> Result<Vec<u8>>;
    fn get_aoc_file_data(&self, name: impl AsRef<Path>) -> Result<Vec<u8>>;
    fn file_exists(&self, name: impl AsRef<Path>) -> bool;
    fn host_path(&self) -> &Path;
}

#[enum_dispatch]
#[derive(Debug)]
enum ROMSource {
    ZArchive,
    // Nsp,
    Unpacked,
}

pub struct GameROMReader {
    source: ROMSource,
    cache: ResourceCache,
}

impl std::fmt::Debug for GameROMReader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameROMReader")
            .field("source", &self.source)
            .field("cache_len", &self.cache.entry_count())
            .finish()
    }
}

impl GameROMReader {
    pub fn from_zarchive(archive_path: impl AsRef<Path>) -> Result<Self> {
        Ok(Self {
            source: ROMSource::ZArchive(ZArchive::new(archive_path)?),
            cache: ResourceCache::new(10_000),
        })
    }

    pub fn from_unpacked_dirs(
        content_dir: impl AsRef<Path>,
        update_dir: impl AsRef<Path>,
        aoc_dir: Option<impl AsRef<Path>>,
    ) -> Result<Self> {
        Ok(Self {
            source: ROMSource::Unpacked(Unpacked::new(content_dir, update_dir, aoc_dir)?),
            cache: ResourceCache::new(10_000),
        })
    }

    pub fn get_resource(&self, name: impl AsRef<Path>) -> Result<Arc<ResourceData>> {
        let name = name
            .as_ref()
            .to_str()
            .ok_or_else(|| ROMError::InvalidPath(name.as_ref().to_string_lossy().into_owned()))?
            .to_owned();
        self.cache
            .get(&name)
            .ok_or_else(|| ROMError::FileNotFound(name, self.source.host_path().to_path_buf()))
    }

    pub fn get_file<T: Into<ResourceData> + TryFrom<ResourceData>>(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Arc<ResourceData>> {
        let canon = canonicalize(path.as_ref());
        let processed = RefCell::new(BTreeMap::new());
        let resource = self
            .cache
            .try_get_with(canon.clone(), || -> uk_content::Result<_> {
                let data = self.source.get_file_data(path.as_ref())?;
                let data = roead::yaz0::decompress_if(data.as_slice())?;
                let resource = ResourceData::from_binary(&canon, data, &processed)?;
                Ok(Arc::new(resource))
            })
            .map_err(|_| {
                ROMError::FileNotFound(
                    path.as_ref().to_string_lossy().to_string(),
                    self.source.host_path().to_path_buf(),
                )
            })?;
        processed.take().into_iter().for_each(|(k, v)| {
            self.cache.insert(k, Arc::new(v));
        });
        Ok(resource)
    }
}
