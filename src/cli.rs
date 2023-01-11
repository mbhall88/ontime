use clap::Parser;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Extract subsets of ONT (Nanopore) reads based on time
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Input fastq file
    #[clap(value_parser = check_path_exists, value_name = "FILE")]
    pub input: PathBuf,
    /// Output file name [default: stdout]
    #[clap(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,
    /// u: uncompressed; b: Bzip2; g: Gzip; l: Lzma
    ///
    /// ontime will attempt to infer the output compression format automatically from the filename
    /// extension. This option is used to override that. If writing to stdout, the default is
    /// uncompressed
    #[clap(short = 'O', long, value_name = "u|b|g|l", value_parser = parse_compression_format, ignore_case=true, hide_possible_values = true)]
    pub output_type: Option<niffler::compression::Format>,
    /// Compression level to use if compressing output
    #[clap(short = 'l', long, value_parser = parse_level, default_value="6", value_name = "1-9")]
    pub compress_level: niffler::Level,
}

/// A collection of custom errors relating to the command line interface for this package.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum CliError {
    /// Indicates that a string cannot be parsed into a [`CompressionFormat`](#compressionformat).
    #[error("{0} is not a valid output format")]
    InvalidCompression(String),
}

pub trait CompressionExt {
    fn from_path<S: AsRef<OsStr> + ?Sized>(p: &S) -> Self;
}

impl CompressionExt for niffler::compression::Format {
    /// Attempts to infer the compression type from the file extension. If the extension is not
    /// known, then Uncompressed is returned.
    fn from_path<S: AsRef<OsStr> + ?Sized>(p: &S) -> Self {
        let path = Path::new(p);
        match path.extension().map(|s| s.to_str()) {
            Some(Some("gz")) => Self::Gzip,
            Some(Some("bz") | Some("bz2")) => Self::Bzip,
            Some(Some("lzma")) => Self::Lzma,
            _ => Self::No,
        }
    }
}

/// A utility function to validate compression level is in allowed range
fn parse_level(s: &str) -> Result<niffler::Level, String> {
    let lvl = match s.parse::<u8>() {
        Ok(1) => niffler::Level::One,
        Ok(2) => niffler::Level::Two,
        Ok(3) => niffler::Level::Three,
        Ok(4) => niffler::Level::Four,
        Ok(5) => niffler::Level::Five,
        Ok(6) => niffler::Level::Six,
        Ok(7) => niffler::Level::Seven,
        Ok(8) => niffler::Level::Eight,
        Ok(9) => niffler::Level::Nine,
        _ => return Err(format!("Compression level {} not in the range 1-9", s)),
    };
    Ok(lvl)
}

fn parse_compression_format(s: &str) -> Result<niffler::compression::Format, CliError> {
    match s {
        "b" | "B" => Ok(niffler::Format::Bzip),
        "g" | "G" => Ok(niffler::Format::Gzip),
        "l" | "L" => Ok(niffler::Format::Lzma),
        "u" | "U" => Ok(niffler::Format::No),
        _ => Err(CliError::InvalidCompression(s.to_string())),
    }
}

/// A utility function that allows the CLI to error if a path doesn't exist
fn check_path_exists<S: AsRef<OsStr> + ?Sized>(s: &S) -> Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if path.exists() {
        Ok(path)
    } else {
        Err(format!("{:?} does not exist", path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_path_exists_it_doesnt() {
        let result = check_path_exists(OsStr::new("fake.path"));
        assert!(result.is_err())
    }

    #[test]
    fn check_path_it_does() {
        let actual = check_path_exists(OsStr::new("Cargo.toml")).unwrap();
        let expected = PathBuf::from("Cargo.toml");
        assert_eq!(actual, expected)
    }

    #[test]
    fn compression_format_from_str() {
        let mut s = "B";
        assert_eq!(parse_compression_format(s).unwrap(), niffler::Format::Bzip);

        s = "g";
        assert_eq!(parse_compression_format(s).unwrap(), niffler::Format::Gzip);

        s = "l";
        assert_eq!(parse_compression_format(s).unwrap(), niffler::Format::Lzma);

        s = "U";
        assert_eq!(parse_compression_format(s).unwrap(), niffler::Format::No);

        s = "a";
        assert_eq!(
            parse_compression_format(s).unwrap_err(),
            CliError::InvalidCompression(s.to_string())
        );
    }

    #[test]
    fn test_in_compress_range() {
        assert!(parse_level("1").is_ok());
        assert!(parse_level("9").is_ok());
        assert!(parse_level("0").is_err());
        assert!(parse_level("10").is_err());
        assert!(parse_level("f").is_err());
        assert!(parse_level("5.5").is_err());
        assert!(parse_level("-3").is_err());
    }

    #[test]
    fn compression_format_from_path() {
        assert_eq!(niffler::Format::from_path("foo.gz"), niffler::Format::Gzip);
        assert_eq!(
            niffler::Format::from_path(Path::new("foo.gz")),
            niffler::Format::Gzip
        );
        assert_eq!(niffler::Format::from_path("baz"), niffler::Format::No);
        assert_eq!(niffler::Format::from_path("baz.fq"), niffler::Format::No);
        assert_eq!(
            niffler::Format::from_path("baz.fq.bz2"),
            niffler::Format::Bzip
        );
        assert_eq!(
            niffler::Format::from_path("baz.fq.bz"),
            niffler::Format::Bzip
        );
        assert_eq!(
            niffler::Format::from_path("baz.fq.lzma"),
            niffler::Format::Lzma
        );
    }
}
