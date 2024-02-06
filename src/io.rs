use crate::cli::CompressionExt;
use anyhow::anyhow;
use needletail::errors::ParseErrorKind::EmptyFile;
use needletail::parse_fastx_file;
use noodles_sam::alignment::record::data::field::Tag;
use noodles_util::alignment::io::Writer;
use ontime::FastxRecordExt;
use std::fs::File;
use std::io::{BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};
use thiserror::Error;
use time::format_description::well_known::Rfc3339;
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

    /// Indicates that some indices we expected to find in the input file weren't found.
    #[error("Some expected indices were not in the input file")]
    IndicesNotFound,

    /// Indicates that writing to the output file failed.
    #[error("Could not write to output file")]
    WriteError { source: anyhow::Error },

    /// Indicates there was an error reading the header of the input file.
    #[error("Could not read the header of the input file")]
    ReadHeaderError { source: anyhow::Error },

    /// Indicates that the alignment file record could not be parsed.
    #[error("Failed to parse alignment record")]
    ParseAlignmentError { source: anyhow::Error },
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

    pub fn extract_reads_in_timeframe_into<T: Write>(
        &self,
        reads_to_keep: &[bool],
        nb_reads_keep: usize,
        write_to: &mut T,
    ) -> Result<(), IOError> {
        let mut reader =
            parse_fastx_file(&self.path).map_err(|source| IOError::ReadError { source })?;
        let mut read_idx: usize = 0;
        let mut nb_reads_written = 0;

        while let Some(record) = reader.next() {
            match record {
                Err(source) => return Err(IOError::ParseError { source }),
                Ok(rec) if reads_to_keep[read_idx] => {
                    rec.write(write_to, None)
                        .map_err(|err| IOError::WriteError {
                            source: anyhow::Error::from(err),
                        })?;
                    nb_reads_written += 1;
                    if nb_reads_keep == nb_reads_written {
                        break;
                    }
                }
                Ok(_) => (),
            }

            read_idx += 1;
        }

        if nb_reads_written == nb_reads_keep {
            Ok(())
        } else {
            Err(IOError::IndicesNotFound)
        }
    }
}

pub trait TimeExt {
    fn start_times(&mut self) -> Result<Vec<PrimitiveDateTime>, IOError>;
    fn extract_reads_in_timeframe_into(
        &mut self,
        reads_to_keep: &[bool],
        nb_reads_keep: usize,
        writer: &mut Writer,
    ) -> Result<(), IOError>;
}

impl TimeExt for noodles_util::alignment::io::reader::Reader<Box<dyn BufRead>> {
    fn start_times(&mut self) -> Result<Vec<PrimitiveDateTime>, IOError> {
        let mut start_times: Vec<PrimitiveDateTime> = vec![];
        let header = self
            .read_header()
            .map_err(|source| IOError::ReadHeaderError {
                source: anyhow::Error::from(source),
            })?;
        let records = self.records(&header);
        let tag = Tag::new(b's', b't');

        for (i, record) in records.enumerate() {
            let record = record.map_err(|source| IOError::ParseAlignmentError {
                source: anyhow! { source.to_string() },
            })?;
            let data = record.data();
            let start_time = data
                .get(&tag)
                .ok_or(IOError::MissingTime(i as u64))?
                .map_err(|_| IOError::MissingTime(i as u64))?;
            let start_time = match start_time {
                noodles_sam::alignment::record::data::field::Value::String(s) => s.to_string(),
                _ => return Err(IOError::MissingTime(i as u64)),
            };
            let start_time = PrimitiveDateTime::parse(&start_time, &Rfc3339)
                .map_err(|_| IOError::MissingTime(i as u64))?;
            start_times.push(start_time);
        }
        Ok(start_times)
    }

    fn extract_reads_in_timeframe_into(
        &mut self,
        reads_to_keep: &[bool],
        nb_reads_keep: usize,
        writer: &mut Writer,
    ) -> Result<(), IOError> {
        let header = self
            .read_header()
            .map_err(|source| IOError::ReadHeaderError {
                source: anyhow::Error::from(source),
            })?;
        let records = self.records(&header);
        let mut nb_reads_written = 0;

        for (i, record) in records.enumerate() {
            let record = record.map_err(|source| IOError::ParseAlignmentError {
                source: anyhow! { source.to_string() },
            })?;
            if reads_to_keep[i] {
                writer
                    .write_record(&header, &record)
                    .map_err(|source| IOError::WriteError {
                        source: anyhow::Error::from(source),
                    })?;
                nb_reads_written += 1;
            }
        }
        if nb_reads_written == nb_reads_keep {
            Ok(())
        } else {
            Err(IOError::IndicesNotFound)
        }
    }
}
