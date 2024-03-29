use bstr::ByteSlice;
use duration_str::DError;
use lazy_static::lazy_static;
use needletail::parser::SequenceRecord;
use regex::bytes::Regex;
use time::format_description::well_known::Rfc3339;
use time::{Duration, PrimitiveDateTime};

lazy_static! {
    pub static ref DATETIME_RE: Regex = Regex::new(r"(start_time=|st:Z:)(?P<time>\S+)\s*").unwrap();
}

pub trait FastxRecordExt {
    fn start_time(&self) -> Option<PrimitiveDateTime>;
}

impl FastxRecordExt for SequenceRecord<'_> {
    fn start_time(&self) -> Option<PrimitiveDateTime> {
        let caps = DATETIME_RE.captures(self.id())?;
        let m = caps.name("time")?;
        let datetime = m.as_bytes().to_str_lossy();
        PrimitiveDateTime::parse(&datetime, &Rfc3339).ok()
    }
}

pub trait DurationExt {
    fn from_str(s: &str) -> Result<Self, DError>
    where
        Self: Sized;
}

impl DurationExt for Duration {
    fn from_str(s: &str) -> Result<Self, DError> {
        if let Some(pos_s) = s.strip_prefix('-') {
            let dur = duration_str::parse_time(pos_s)?;
            Ok(-1 * dur)
        } else {
            duration_str::parse_time(s)
        }
    }
}

pub fn valid_indices(
    timestamps: &[PrimitiveDateTime],
    earliest: &PrimitiveDateTime,
    latest: &PrimitiveDateTime,
) -> (Vec<bool>, usize) {
    let mut to_keep: Vec<bool> = vec![false; timestamps.len()];
    let mut nb_reads_to_keep = 0;
    timestamps.iter().enumerate().for_each(|(i, t)| {
        if earliest <= t && t <= latest {
            to_keep[i] = true;
            nb_reads_to_keep += 1;
        }
    });

    (to_keep, nb_reads_to_keep)
}

#[cfg(test)]
mod tests {
    use super::*;
    use needletail::parse_fastx_file;
    use std::io::Write;
    use tempfile::Builder;
    use time::macros::{date, time};
    use time::Duration;

    #[test]
    fn test_no_start_time() {
        let text = "@read1\nA\n+\n1";
        let mut file = Builder::new().suffix(".fa").tempfile().unwrap();
        file.write_all(text.as_bytes()).unwrap();

        let mut reader = parse_fastx_file(file.path()).unwrap();
        let rec = reader.next().unwrap();
        let record = rec.unwrap();

        let actual = record.start_time();
        let expected = None;

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_start_time_old_valid() {
        let text = "@read1 ch=352 start_time=2022-12-12T18:39:27Z model_version_id=2021\nA\n+\n1";
        let mut file = Builder::new().suffix(".fa").tempfile().unwrap();
        file.write_all(text.as_bytes()).unwrap();

        let mut reader = parse_fastx_file(file.path()).unwrap();
        let rec = reader.next().unwrap();
        let record = rec.unwrap();

        let actual = record.start_time().unwrap();
        let expected = PrimitiveDateTime::new(date!(2022 - 12 - 12), time!(18:39:27));

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_start_time_offset_valid() {
        let text =
            "@read1 ch=352 start_time=2021-07-08T17:47:25+01:00 model_version_id=2021\nA\n+\n1";
        let mut file = Builder::new().suffix(".fa").tempfile().unwrap();
        file.write_all(text.as_bytes()).unwrap();

        let mut reader = parse_fastx_file(file.path()).unwrap();
        let rec = reader.next().unwrap();
        let record = rec.unwrap();

        let actual = record.start_time().unwrap();
        let expected = PrimitiveDateTime::new(date!(2021 - 07 - 08), time!(17:47:25));

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_start_time_offset_with_micro_valid() {
        let text = "@read1 ch=352 start_time=2021-07-08T17:47:25.558027+01:00 model_version_id=2021\nA\n+\n1";
        let mut file = Builder::new().suffix(".fa").tempfile().unwrap();
        file.write_all(text.as_bytes()).unwrap();

        let mut reader = parse_fastx_file(file.path()).unwrap();
        let rec = reader.next().unwrap();
        let record = rec.unwrap();

        let actual = record.start_time().unwrap();
        let expected = PrimitiveDateTime::new(date!(2021 - 07 - 08), time!(17:47:25.558027));

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_start_time_invalid_without_z() {
        let text = "@read1 ch=352 start_time=2022-12-12T18:39:27 model_version_id=2021\nA\n+\n1";
        let mut file = Builder::new().suffix(".fa").tempfile().unwrap();
        file.write_all(text.as_bytes()).unwrap();

        let mut reader = parse_fastx_file(file.path()).unwrap();
        let rec = reader.next().unwrap();
        let record = rec.unwrap();

        let actual = record.start_time();
        assert!(actual.is_none())
    }

    #[test]
    fn test_start_time_invalid() {
        let text = "@read1 ch=352 start_time=2022-12-12T18:39Z model_version_id=2021\nA\n+\n1";
        let mut file = Builder::new().suffix(".fa").tempfile().unwrap();
        file.write_all(text.as_bytes()).unwrap();

        let mut reader = parse_fastx_file(file.path()).unwrap();
        let rec = reader.next().unwrap();
        let record = rec.unwrap();

        let actual = record.start_time();
        let expected = None;

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_bam_tag_start_time_is_valid() {
        let text = "@read1 st:Z:2023-08-07T13:14:42.356+00:00\nA\n+\n1";
        let mut file = Builder::new().suffix(".fa").tempfile().unwrap();
        file.write_all(text.as_bytes()).unwrap();

        let mut reader = parse_fastx_file(file.path()).unwrap();
        let rec = reader.next().unwrap();
        let record = rec.unwrap();

        let actual = record.start_time().unwrap();
        let expected = PrimitiveDateTime::new(date!(2023 - 08 - 07), time!(13:14:42.356));

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_duration_from_str_negative() {
        let s = "-1h";
        let actual = Duration::from_str(s).unwrap();
        let expected = Duration::hours(-1);

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_duration_from_str_negative_invalid() {
        let s = "1d-1h";
        let actual = Duration::from_str(s);
        assert!(actual.is_err())
    }

    #[test]
    fn test_duration_from_str() {
        let s = "11h30min";
        let actual = Duration::from_str(s).unwrap();
        let expected = Duration::seconds(41_400);

        assert_eq!(actual, expected)
    }

    #[test]
    fn test_duration_from_str_invalid() {
        let s = "11h30min12foo";
        let actual = Duration::from_str(s);
        assert!(actual.is_err())
    }
}
