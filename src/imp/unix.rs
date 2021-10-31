use crate::{Error, FinalPath, TempFilePath};
use std::{
    fs,
    io::{self, ErrorKind},
};

impl From<nix::Error> for crate::Error {
    fn from(error: nix::Error) -> Self {
        crate::Error::Io(io::Error::new(ErrorKind::Other, error.desc()))
    }
}

pub(crate) fn commit(from: &TempFilePath, to: &FinalPath) -> Result<(), Error> {
    let from = from.0.as_path();
    let to = to.0.as_path();
    fs::rename(from, to)?;

    let to_parent = to.parent().ok_or(Error::InvalidParentFinalPath)?;
    debug_assert!(to_parent.is_dir());

    let to_parent = fs::File::open(to_parent)?;

    use std::os::unix::io::AsRawFd;
    nix::unistd::fsync(to_parent.as_raw_fd())?;

    Ok(())
}
