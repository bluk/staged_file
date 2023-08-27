use crate::{BoxError, Error, FinalPath, TempFilePath};
use std::fs;

impl From<nix::Error> for crate::Error {
    fn from(error: nix::Error) -> Self {
        crate::Error::Other(BoxError(Box::new(error)))
    }
}

pub(crate) fn commit(from: &TempFilePath, to: &FinalPath) -> Result<(), Error> {
    use std::os::unix::io::AsRawFd;

    let from = from.0.as_path();
    let to = to.0.as_path();
    fs::rename(from, to)?;

    let to_parent = to.parent().ok_or(Error::InvalidParentFinalPath)?;
    debug_assert!(to_parent.is_dir());

    let to_parent = fs::File::open(to_parent)?;

    nix::unistd::fsync(to_parent.as_raw_fd())?;

    Ok(())
}
