use alloc::string::String;
use core::convert::Infallible;
use thiserror::Error;

/// An error from reading or building an EROFS image.
///
/// `E` is the error an [`AsyncImage`](crate::backend::AsyncImage) backend
/// reports for a failed read, carried unchanged in [`Error::Image`]. Every
/// other source of errors, and every backend that cannot fail on its own,
/// uses the default `Infallible`.
#[derive(Debug, Error)]
pub enum Error<E = Infallible> {
    #[error("invalid super block: {0}")]
    InvalidSuperblock(String),

    #[error("invalid dirent file type: {0}")]
    InvalidDirentFileType(u8),

    #[error("path not found: {0}")]
    PathNotFound(String),

    #[error("not a file: {0}")]
    NotAFile(String),

    #[error("not a directory: {0}")]
    NotADirectory(String),

    #[error("out of bounds: {0}")]
    OutOfBounds(String),

    #[error("integer overflow: {0}")]
    Overflow(&'static str),

    #[error("not a symbolic link: {0}")]
    NotASymlink(u64),

    #[error("binread error: {0}")]
    BinRead(#[cfg_attr(feature = "std", from)] binrw::Error),

    #[error("out of range {0} of {1}")]
    OutOfRange(u64, u64),

    #[error("{0} not supported yet")]
    NotSupported(String),

    #[error("corrupted data: {0}")]
    CorruptedData(String),

    #[cfg(feature = "std")]
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[cfg(feature = "opendal")]
    #[error("opendal error: {0}")]
    Opendal(#[from] opendal::Error),

    /// The image backend failed to read.
    #[error(transparent)]
    Image(E),
}

impl Error {
    /// Carries an error that no image read produced into a reader whose
    /// backend reports `E`.
    pub(crate) fn widen<E>(self) -> Error<E> {
        match self {
            Self::InvalidSuperblock(message) => Error::InvalidSuperblock(message),
            Self::InvalidDirentFileType(kind) => Error::InvalidDirentFileType(kind),
            Self::PathNotFound(path) => Error::PathNotFound(path),
            Self::NotAFile(message) => Error::NotAFile(message),
            Self::NotADirectory(message) => Error::NotADirectory(message),
            Self::OutOfBounds(message) => Error::OutOfBounds(message),
            Self::Overflow(what) => Error::Overflow(what),
            Self::NotASymlink(nid) => Error::NotASymlink(nid),
            Self::BinRead(error) => Error::BinRead(error),
            Self::OutOfRange(end, size) => Error::OutOfRange(end, size),
            Self::NotSupported(what) => Error::NotSupported(what),
            Self::CorruptedData(message) => Error::CorruptedData(message),
            #[cfg(feature = "std")]
            Self::Io(error) => Error::Io(error),
            #[cfg(feature = "opendal")]
            Self::Opendal(error) => Error::Opendal(error),
            Self::Image(never) => match never {},
        }
    }
}

/// Widens a result that no image read produced; see [`Error::widen`].
pub trait Widen<T> {
    fn widen<E>(self) -> Result<T, E>;
}

impl<T> Widen<T> for Result<T> {
    fn widen<E>(self) -> Result<T, E> {
        self.map_err(Error::widen)
    }
}

// binrw only implements the Error trait with std; preserve the value without
// treating it as an error source in no_std builds.
#[cfg(not(feature = "std"))]
impl<E> From<binrw::Error> for Error<E> {
    fn from(error: binrw::Error) -> Self {
        Self::BinRead(error)
    }
}

pub type Result<T, E = Infallible> = core::result::Result<T, Error<E>>;
