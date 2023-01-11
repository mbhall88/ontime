use bstr::ByteSlice;
use lazy_static::lazy_static;
use needletail::parser::SequenceRecord;
use regex::bytes::Regex;
use time::PrimitiveDateTime;
use time::format_description::well_known::Iso8601;

pub trait FastxRecordExt {
    fn start_time(&self) -> Option<PrimitiveDateTime>;
}

impl FastxRecordExt for SequenceRecord<'_> {
    fn start_time(&self) -> Option<PrimitiveDateTime> {
        lazy_static! {
            static ref RE: Regex =
                Regex::new(r"start_time=(?P<time>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})").unwrap();
        }
        let Some(caps) = RE.captures(self.id()) else {return None};
        let Some(m) = caps.name("time") else {return None};
        let datetime = m.as_bytes().to_str_lossy();
        PrimitiveDateTime::parse(&*datetime, &Iso8601::DEFAULT).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use needletail::parse_fastx_file;
    use std::io::{Read, Write};
    use tempfile::Builder;
    use time::macros::{date, time};

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
    fn test_start_time_valid() {
        let text = "@read1 ch=352 start_time=2022-12-12T18:39:27Z model_version_id=2021\nA\n+\n1";
        let mut file = Builder::new().suffix(".fa").tempfile().unwrap();
        file.write_all(text.as_bytes()).unwrap();

        let mut reader = parse_fastx_file(file.path()).unwrap();
        let rec = reader.next().unwrap();
        let record = rec.unwrap();

        let actual = record.start_time().unwrap();
        let expected = PrimitiveDateTime::new(date!(2022-12-12), time!(18:39:27));

        assert_eq!(actual, expected)
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
}
