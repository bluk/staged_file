//! `StagedFile` helps write data to a temporary file, and then gives the option
//! to commit the temporary file to a desired final file path.
//!
//! First, a staged file is [created](`StagedFile::with_final_path()`) with the
//! desired final path of the file. The staged file starts as a newly created
//! temporary file.
//!
//! Then, data is written out by using the `StagedFile` as a `File` ref.
//!
//! Finally, the file is commited by calling [`StagedFile::commit()`]. If the
//! temporary file contents are not to be committed, then let the `StagedFile` be
//! dropped without calling `commit`.
//!
//! ```no_run
//! use staged_file::StagedFile;
//! use std::fs::File;
//! use std::io::{prelude::*, LineWriter};
//! use std::path::Path;
//!
//! # fn main() -> Result<(), staged_file::Error> {
//! let final_path = Path::new("/a/file/path");
//! let staged_file = StagedFile::with_final_path(&final_path)?;
//!
//! let text = b"Hello World!";
//!
//! {
//!     let mut line_writer = LineWriter::new(&staged_file);
//!     line_writer.write_all(text)?;
//!     line_writer.flush()?;
//! }
//!
//! staged_file.commit()?;
//!
//! assert_eq!(std::fs::read(final_path)?, text);
//! # Ok(())
//! # }
//! ```
use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    io,
    path::{Path, PathBuf},
    result,
};

/// A type erased boxed error.
///
/// Used for other implementation errors.
#[derive(Debug)]
pub struct BoxError(Box<dyn std::error::Error + Send + Sync + 'static>);

impl Display for BoxError {
    fn fmt(&self, f: &mut Formatter<'_>) -> result::Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

impl std::error::Error for BoxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.0.source()
    }
}

/// Possible errors when creating and committing the staged file.
#[derive(Debug)]
pub enum Error {
    /// The final path for the file is invalid.
    InvalidFinalPath,
    /// The parent directory of the final path is not valid (e.g. cannot be accessed or determined).
    InvalidParentFinalPath,
    /// An I/O error.
    Io(io::Error),
    /// All other errors.
    Other(BoxError),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> result::Result<(), fmt::Error> {
        match self {
            Error::InvalidFinalPath => write!(f, "invalid final path"),
            Error::InvalidParentFinalPath => write!(f, "invalid parent final path"),
            Error::Io(e) => e.fmt(f),
            Error::Other(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::InvalidFinalPath => None,
            Error::InvalidParentFinalPath => None,
            Error::Io(e) => Some(e),
            Error::Other(e) => Some(e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::Io(error)
    }
}

#[derive(Debug)]
struct TempFilePath(PathBuf);

#[derive(Debug)]
struct FinalPath(PathBuf);

fn final_path_parent(final_path: &Path) -> Result<&Path, Error> {
    final_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .ok_or(Error::InvalidParentFinalPath)
}

#[derive(Debug)]
enum State {
    Staged {
        temp_file: File,
        temp_dir: tempfile::TempDir,
        temp_file_path: TempFilePath,
    },
    Committed,
}

/// Creates a temporary file which can then be committed to a final path.
#[derive(Debug)]
pub struct StagedFile {
    final_path: FinalPath,
    state: State,
}

impl Drop for StagedFile {
    fn drop(&mut self) {
        let mut state = State::Committed;
        std::mem::swap(&mut self.state, &mut state);
        if let State::Staged {
            temp_file,
            temp_dir,
            temp_file_path: _temp_file_path,
        } = state
        {
            drop(temp_file);
            drop(temp_dir);
        }
    }
}

impl StagedFile {
    /// Instantiates a new staged file with the desired final path.
    ///
    /// The desired final path is where the file contents should be if the staged
    /// file is [committed](`StagedFile::commit()`).
    ///
    /// A temporary directory and file are created during this function call.
    ///
    /// # Important
    ///
    /// If a file exists at the desired final file path, it will be overwritten.
    ///
    /// # Errors
    ///
    /// If the final path is invalid (e.g. is a directory) or if the final path's
    /// parent directory cannot be determined, an [`Error`] will be returned.
    ///
    /// Any I/O error which occurs when creating the temporary directory or file
    /// will also be returned.
    pub fn with_final_path<P>(final_path: P) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        Self::with_final_path_and_temp_dir_prefix(final_path, None)
    }

    /// Instantiates a new staged file with the desired final path and a
    /// temporary directory prefix.
    ///
    /// The desired final path is where the file contents should be if the staged
    /// file is [committed](`StagedFile::commit()`).
    ///
    /// A temporary directory and file are created during this function call.
    /// The temporary directory will have the prefix given or a default prefix
    /// will be chosen.
    ///
    /// # Important
    ///
    /// If a file exists at the desired final file path, it will be overwritten.
    ///
    /// # Errors
    ///
    /// If the final path is invalid (e.g. is a directory) or if the final path's
    /// parent directory cannot be determined, an [`Error`] will be returned.
    ///
    /// Any I/O error which occurs when creating the temporary directory or file
    /// will also be returned.
    pub fn with_final_path_and_temp_dir_prefix<P>(
        final_path: P,
        temp_dir_prefix: Option<&str>,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let final_path = final_path.as_ref();
        if final_path.is_dir() {
            return Err(Error::InvalidFinalPath);
        }
        let temp_dir = tempfile::Builder::new()
            .prefix(temp_dir_prefix.unwrap_or(".staged"))
            .tempdir_in(final_path_parent(final_path)?)?;
        let temp_file_path = temp_dir
            .path()
            .join(final_path.file_name().ok_or(Error::InvalidFinalPath)?);
        let temp_file = File::create(&temp_file_path)?;

        Ok(Self {
            final_path: FinalPath(final_path.to_path_buf()),
            state: State::Staged {
                temp_file,
                temp_dir,
                temp_file_path: TempFilePath(temp_file_path),
            },
        })
    }

    /// Commits the temporary file contents into the desired final path.
    ///
    /// If the contents should *not* be committed, then allow the `StagedFile` to
    /// be dropped without calling commit.
    ///
    /// # Important
    ///
    /// If a file exists at the desired final file path, it will be overwritten.
    ///
    /// # Errors
    ///
    /// Any I/O errors encountered will be returned.
    pub fn commit(mut self) -> Result<(), Error> {
        let mut state = State::Committed;
        std::mem::swap(&mut self.state, &mut state);
        if let State::Staged {
            temp_file,
            temp_dir,
            temp_file_path,
        } = state
        {
            temp_file.sync_all()?;
            // Explicit drop to remove any open file descriptors so temp dir can be deleted
            drop(temp_file);

            imp::commit(&temp_file_path, &self.final_path)?;

            drop(temp_dir);

            Ok(())
        } else {
            unreachable!()
        }
    }

    #[inline]
    fn as_file(&self) -> &File {
        if let State::Staged { ref temp_file, .. } = self.state {
            temp_file
        } else {
            unreachable!()
        }
    }
}

impl io::Write for StagedFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.as_file().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.as_file().flush()
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        self.as_file().write_vectored(bufs)
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.as_file().write_all(buf)
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        self.as_file().write_fmt(fmt)
    }
}

impl io::Write for &StagedFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.as_file().write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.as_file().flush()
    }

    fn write_vectored(&mut self, bufs: &[io::IoSlice<'_>]) -> io::Result<usize> {
        self.as_file().write_vectored(bufs)
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.as_file().write_all(buf)
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> io::Result<()> {
        self.as_file().write_fmt(fmt)
    }
}

impl io::Seek for StagedFile {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.as_file().seek(pos)
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        self.as_file().stream_position()
    }
}

impl io::Seek for &StagedFile {
    fn seek(&mut self, pos: io::SeekFrom) -> io::Result<u64> {
        self.as_file().seek(pos)
    }

    fn stream_position(&mut self) -> io::Result<u64> {
        self.as_file().stream_position()
    }
}

impl io::Read for StagedFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.as_file().read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        self.as_file().read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.as_file().read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.as_file().read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.as_file().read_exact(buf)
    }
}

impl io::Read for &StagedFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.as_file().read(buf)
    }

    fn read_vectored(&mut self, bufs: &mut [io::IoSliceMut<'_>]) -> io::Result<usize> {
        self.as_file().read_vectored(bufs)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        self.as_file().read_to_end(buf)
    }

    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {
        self.as_file().read_to_string(buf)
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        self.as_file().read_exact(buf)
    }
}

pub(crate) mod imp;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn commit_staged_file() {
        use std::io::prelude::*;
        use std::io::LineWriter;

        let temp_dir = tempfile::tempdir().unwrap();
        let final_path = temp_dir.path().join("test1");
        let staged_file = StagedFile::with_final_path(&final_path).unwrap();

        let text = b"Hello World!";

        {
            let mut line_writer = LineWriter::new(&staged_file);
            line_writer.write_all(text).unwrap();
            line_writer.flush().unwrap();
        }

        staged_file.commit().unwrap();

        assert!(final_path.exists());
        assert_eq!(std::fs::read(final_path).unwrap(), text);
    }

    #[test]
    fn no_commit_staged_file() {
        use std::io::prelude::*;
        use std::io::LineWriter;

        let temp_dir = tempfile::tempdir().unwrap();
        let final_path = temp_dir.path().join("test2");
        let staged_file = StagedFile::with_final_path(&final_path).unwrap();

        let text = b"Hello World!";

        let mut line_writer = LineWriter::new(&staged_file);
        line_writer.write_all(text).unwrap();
        line_writer.flush().unwrap();

        assert!(!final_path.exists());
    }
}
