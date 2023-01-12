use crate::cli::CompressionExt;
use needletail::errors::ParseErrorKind::EmptyFile;
use needletail::parse_fastx_file;
use ontime::FastxRecordExt;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use time::PrimitiveDateTime;

/// A `Struct` used for seamlessly dealing with either compressed or uncompressed fasta/fastq files.
#[derive(Debug, PartialEq, Eq)]
pub struct Fastx {
    /// The path for the file.
    path: PathBuf,
}

/// A collection of custom errors relating to the working with files for this package.
#[derive(Error, Debug)]
pub enum IOError {
    /// Indicates that the specified input file could not be opened/read.
    #[error("Read error")]
    ReadError {
        source: needletail::errors::ParseError,
    },

    /// Indicates that a sequence record could not be parsed.
    #[error("Failed to parse record")]
    ParseError {
        source: needletail::errors::ParseError,
    },

    /// Indicates that the specified output file could not be created.
    #[error("Output file could not be created")]
    CreateError { source: std::io::Error },

    /// The fastq record is missing the start time
    #[error("Missing start_time in fastq record start at line {0}")]
    MissingTime(u64),

    /// Indicates and error trying to create the compressor
    #[error(transparent)]
    CompressOutputError(#[from] niffler::Error),
}

impl Fastx {
    /// Create a `Fastx` object from a `std::path::Path`.
    ///
    /// # Example
    ///
    /// ```rust
    /// let path = std::path::Path::new("input.fa.gz");
    /// let fastx = Fastx::from_path(path);
    /// ```
    pub fn from_path(path: &Path) -> Self {
        Fastx {
            path: path.to_path_buf(),
        }
    }
    /// Create the file associated with this `Fastx` object for writing.
    ///
    /// # Errors
    /// If the file cannot be created then an `Err` containing a variant of [`FastxError`](#fastxerror) is
    /// returned.
    ///
    /// # Example
    ///
    /// ```rust
    /// let path = std::path::Path::new("output.fa");
    /// let fastx = Fastx{ path };
    /// { // this scoping means the file handle is closed afterwards.
    ///     let file_handle = fastx.create(6, None)?;
    ///     write!(file_handle, ">read1\nACGT\n")?
    /// }
    /// ```
    pub fn create(
        &self,
        compression_lvl: niffler::compression::Level,
        compression_fmt: Option<niffler::compression::Format>,
    ) -> Result<Box<dyn Write>, IOError> {
        let file = File::create(&self.path).map_err(|source| IOError::CreateError { source })?;
        let file_handle = Box::new(BufWriter::new(file));
        let fmt = match compression_fmt {
            None => niffler::Format::from_path(&self.path),
            Some(f) => f,
        };
        niffler::get_writer(file_handle, fmt, compression_lvl).map_err(IOError::CompressOutputError)
    }
    /// Returns a vector containing the start time of each read.
    ///
    /// # Errors
    /// If the file cannot be opened or there is an issue parsing any records then an
    /// `Err` containing a variant of [`IOError`](#ioerror) is returned.
    pub fn start_times(&self) -> Result<Vec<PrimitiveDateTime>, IOError> {
        let mut start_times: Vec<PrimitiveDateTime> = vec![];
        let mut reader = match parse_fastx_file(&self.path) {
            Ok(rdr) => rdr,
            Err(e) if e.kind == EmptyFile => return Ok(start_times),
            Err(source) => return Err(IOError::ReadError { source }),
        };

        while let Some(record) = reader.next() {
            match record {
                Ok(rec) => {
                    let start_time = match rec.start_time() {
                        Some(t) => t,
                        None => return Err(IOError::MissingTime(rec.start_line_number())),
                    };
                    start_times.push(start_time)
                }
                Err(err) => return Err(IOError::ParseError { source: err }),
            }
        }
        Ok(start_times)
    }
}