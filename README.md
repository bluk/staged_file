# StagedFile

`StagedFile` helps write data to a temporary file, and then gives the option
to commit the temporary file to a desired final file path.

If a file exists at the desired final file path, the file will be overwritten
during a commit function call currently.

Only UNIX is currently supported.

## Installation

```toml
[dependencies]
staged_file = "0.4"
```

## Example

```rust
use staged_file::StagedFile;
use std::fs::File;
use std::io::{prelude::*, LineWriter};
use std::path::Path;

let final_path = Path::new("/a/file/path");
let staged_file = StagedFile::with_final_path(&final_path)?;

let text = b"Hello World!";

{
    // The LineWriter code is in a block so that `&staged_file` is not considered
    // borrowed at the end of the block. Another way to get back the
    // `staged_file` is to call `line_writer.into_inner()`.
    let mut line_writer = LineWriter::new(&staged_file);
    line_writer.write_all(text)?;
    line_writer.flush()?;
}

staged_file.commit()?;

assert_eq!(std::fs::read(final_path)?, text);
```

If the `commit()` method is not called, then the staged file contents are
discarded.

## Other Libraries

* [tempfile][tempfile]

The library is used as a dependency in this crate to create temporary directories.

* [atomicwrites][atomicwrites]

A cross platform atomic file writes library.

## License

Licensed under either of [Apache License, Version 2.0][LICENSE_APACHE] or [MIT
License][LICENSE_MIT] at your option.

### Contributions

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

[LICENSE_APACHE]: LICENSE-APACHE
[LICENSE_MIT]: LICENSE-MIT
[tempfile]: https://github.com/Stebalien/tempfile
[atomicwrites]: https://github.com/untitaker/rust-atomicwrites
