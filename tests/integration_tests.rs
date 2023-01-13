use assert_cmd::Command;
use indoc::indoc;
use std::io::Write;

const BIN: &str = "ontime";

#[test]
fn input_file_does_not_exist() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let err_msg = cmd.arg("nonexistent.fa").unwrap_err().to_string();

    assert!(err_msg.contains("does not exist"));

    Ok(())
}

#[test]
fn trying_to_create_output_in_nonexistent_dir() -> Result<(), Box<dyn std::error::Error>> {
    let text = ">s0\nACGT\n>s1\nGCCC\n";
    let mut file = tempfile::Builder::new().suffix(".fa").tempfile().unwrap();
    file.write_all(text.as_bytes()).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let p = "foo/bar/aln.fa";

    let err_msg = cmd
        .args(["-o", p, file.path().to_str().unwrap()])
        .unwrap_err()
        .to_string();

    assert!(err_msg.contains("Failed to create the output file"));

    Ok(())
}

#[test]
fn input_has_no_start_times() -> Result<(), Box<dyn std::error::Error>> {
    let text = indoc! {b"@s0
    A
    +
    1
    @s1
    C
    +
    1
    "};
    let mut file = tempfile::Builder::new().suffix(".fq").tempfile().unwrap();
    file.write_all(text).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let err_msg = cmd.args([file.path()]).unwrap_err().to_string();

    assert!(err_msg.contains("Failed to parse a start time"));

    Ok(())
}

#[test]
fn input_has_one_read_with_no_start_time() -> Result<(), Box<dyn std::error::Error>> {
    let text = indoc! {b"@s0 start_time=2022-12-12T18:00:00Z
    A
    +
    1
    @s1
    C
    +
    1
    "};
    let mut file = tempfile::Builder::new().suffix(".fq").tempfile().unwrap();
    file.write_all(text).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let err_msg = cmd.args([file.path()]).unwrap_err().to_string();

    assert!(err_msg.contains("Failed to parse a start time"));

    Ok(())
}

#[test]
fn input_has_one_read_with_no_valid_start_time() -> Result<(), Box<dyn std::error::Error>> {
    let text = indoc! {b"@s0 start_time=2022-12-12T18:00:00Z
    A
    +
    1
    @s1 start_time=12:00:00Z
    C
    +
    1
    "};
    let mut file = tempfile::Builder::new().suffix(".fq").tempfile().unwrap();
    file.write_all(text).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let err_msg = cmd.args([file.path()]).unwrap_err().to_string();

    assert!(err_msg.contains("Failed to parse a start time"));

    Ok(())
}

#[test]
fn no_from_and_to_gets_all_reads() -> Result<(), Box<dyn std::error::Error>> {
    let text = indoc! {b"@s0 start_time=2022-12-12T18:00:00Z
    A
    +
    1
    @s1 start_time=2022-12-12T12:00:00Z
    C
    +
    1
    "};
    let mut file = tempfile::Builder::new().suffix(".fq").tempfile().unwrap();
    file.write_all(text).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let output = cmd.args([file.path()]).unwrap().stdout;
    let expected = text;

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn timeframe_excludes_all_times() -> Result<(), Box<dyn std::error::Error>> {
    let text = indoc! {b"@s0 start_time=2022-12-12T18:00:00Z
    A
    +
    1
    @s1 start_time=2022-12-12T12:00:00Z
    C
    +
    1
    "};
    let mut file = tempfile::Builder::new().suffix(".fq").tempfile().unwrap();
    file.write_all(text).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let output = cmd
        .args(["-f", "400h", "-t", "500h", file.path().to_str().unwrap()])
        .unwrap()
        .stdout;

    assert!(output.is_empty());

    Ok(())
}

#[test]
fn timeframe_includes_only_earliest() -> Result<(), Box<dyn std::error::Error>> {
    let text = indoc! {b"@s0 start_time=2022-12-12T18:00:00Z
    A
    +
    1
    @s1 start_time=2022-12-12T12:00:00Z
    C
    +
    1
    "};
    let mut file = tempfile::Builder::new().suffix(".fq").tempfile().unwrap();
    file.write_all(text).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let output = cmd
        .args(["-t", "1m", file.path().to_str().unwrap()])
        .unwrap()
        .stdout;

    let expected = indoc! {b"@s1 start_time=2022-12-12T12:00:00Z
    C
    +
    1
    "};

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn timeframe_includes_only_latest() -> Result<(), Box<dyn std::error::Error>> {
    let text = indoc! {b"@s0 start_time=2022-12-12T18:00:00Z
    A
    +
    1
    @s1 start_time=2022-12-12T12:00:00Z
    C
    +
    1
    "};
    let mut file = tempfile::Builder::new().suffix(".fq").tempfile().unwrap();
    file.write_all(text).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let output = cmd
        .args(["-f", "1m", file.path().to_str().unwrap()])
        .unwrap()
        .stdout;

    let expected = indoc! {b"@s0 start_time=2022-12-12T18:00:00Z
    A
    +
    1
    "};

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn timeframe_excludes_earliest_and_latest() -> Result<(), Box<dyn std::error::Error>> {
    let text = indoc! {b"@s0 start_time=2022-12-12T18:00:00Z
    A
    +
    1
    @s2 start_time=2022-12-12T14:00:00Z
    G
    +
    4
    @s1 start_time=2022-12-12T12:00:00Z
    C
    +
    1
    "};
    let mut file = tempfile::Builder::new().suffix(".fq").tempfile().unwrap();
    file.write_all(text).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let output = cmd
        .args(["-f", "1m", "-t", "-2min", file.path().to_str().unwrap()])
        .unwrap()
        .stdout;

    let expected = indoc! {b"@s2 start_time=2022-12-12T14:00:00Z
    G
    +
    4
    "};

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn timeframe_excludes_earliest_and_latest_using_timestamp() -> Result<(), Box<dyn std::error::Error>>
{
    let text = indoc! {b"@s0 start_time=2022-12-12T18:00:00Z
    A
    +
    1
    @s2 start_time=2022-12-12T14:00:00Z
    G
    +
    4
    @s1 start_time=2022-12-12T12:00:00Z
    C
    +
    1
    "};
    let mut file = tempfile::Builder::new().suffix(".fq").tempfile().unwrap();
    file.write_all(text).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let output = cmd
        .args([
            "-f",
            "2022-12-12T13:00:00Z",
            "-t",
            "2022-12-12T15:00:00Z",
            file.path().to_str().unwrap(),
        ])
        .unwrap()
        .stdout;

    let expected = indoc! {b"@s2 start_time=2022-12-12T14:00:00Z
    G
    +
    4
    "};

    assert_eq!(output, expected);

    Ok(())
}

#[test]
fn earliest_is_after_latest() -> Result<(), Box<dyn std::error::Error>> {
    let text = indoc! {b"@s0 start_time=2022-12-12T18:00:00Z
    A
    +
    1
    @s1 start_time=2022-12-12T12:00:00Z
    C
    +
    1
    "};
    let mut file = tempfile::Builder::new().suffix(".fq").tempfile().unwrap();
    file.write_all(text).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let output = cmd
        .args(["-f", "1w", file.path().to_str().unwrap()])
        .unwrap_err()
        .to_string();

    assert!(output.contains("earliest timestamp is after the latest"));

    Ok(())
}

#[test]
fn latest_is_before_earliest() -> Result<(), Box<dyn std::error::Error>> {
    let text = indoc! {b"@s0 start_time=2022-12-12T18:00:00Z
    A
    +
    1
    @s1 start_time=2022-12-12T12:00:00Z
    C
    +
    1
    "};
    let mut file = tempfile::Builder::new().suffix(".fq").tempfile().unwrap();
    file.write_all(text).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let output = cmd
        .args(["-t", "-1w", file.path().to_str().unwrap()])
        .unwrap_err()
        .to_string();

    assert!(output.contains("earliest timestamp is after the latest"));

    Ok(())
}

#[test]
fn output_is_gzip_compressed() -> Result<(), Box<dyn std::error::Error>> {
    let text = indoc! {b"@s0 start_time=2022-12-12T18:00:00Z
    A
    +
    1
    @s1 start_time=2022-12-12T12:00:00Z
    C
    +
    1
    "};
    let mut file = tempfile::Builder::new().suffix(".fq").tempfile().unwrap();
    file.write_all(text).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let output = cmd
        .args(["-O", "g", file.path().to_str().unwrap()])
        .unwrap()
        .stdout;

    let (_, expected) = niffler::sniff(Box::new(&output[..])).unwrap();

    assert_eq!(niffler::Format::Gzip, expected);

    Ok(())
}

#[test]
fn output_is_bzip2_compressed() -> Result<(), Box<dyn std::error::Error>> {
    let text = indoc! {b"@s0 start_time=2022-12-12T18:00:00Z
    A
    +
    1
    @s1 start_time=2022-12-12T12:00:00Z
    C
    +
    1
    "};
    let mut file = tempfile::Builder::new().suffix(".fq").tempfile().unwrap();
    file.write_all(text).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let output = cmd
        .args(["-O", "b", file.path().to_str().unwrap()])
        .unwrap()
        .stdout;

    let (_, expected) = niffler::sniff(Box::new(&output[..])).unwrap();

    assert_eq!(niffler::Format::Bzip, expected);

    Ok(())
}

#[test]
fn output_is_lzma_compressed() -> Result<(), Box<dyn std::error::Error>> {
    let text = indoc! {b"@s0 start_time=2022-12-12T18:00:00Z
    A
    +
    1
    @s1 start_time=2022-12-12T12:00:00Z
    C
    +
    1
    "};
    let mut file = tempfile::Builder::new().suffix(".fq").tempfile().unwrap();
    file.write_all(text).unwrap();
    let mut cmd = Command::cargo_bin(BIN).unwrap();
    let output = cmd
        .args(["-O", "l", file.path().to_str().unwrap()])
        .unwrap()
        .stdout;

    let (_, expected) = niffler::sniff(Box::new(&output[..])).unwrap();

    assert_eq!(niffler::Format::Lzma, expected);

    Ok(())
}
