mod cli;
mod io;

use crate::cli::Cli;
use crate::io::Fastx;
use crate::io::TimeExt;
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use env_logger::Builder;
use itertools::Itertools;
use itertools::MinMaxResult::{MinMax, NoElements, OneElement};
use log::info;
use log::LevelFilter;
use ontime::{valid_indices, DurationExt};
use std::io::stdout;
use time::format_description::well_known::Rfc3339;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{Duration, PrimitiveDateTime};

const TIME_FMT: &[FormatItem<'_>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond]Z");

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum FileFormat {
    Alignment,
    Fastx,
}

fn main() -> Result<()> {
    let args = Cli::parse();
    // setup logging
    let mut log_builder = Builder::new();
    log_builder
        .filter(None, LevelFilter::Info)
        .format_module_path(false)
        .format_target(false)
        .init();

    let input_format = match &args.input.extension().and_then(|ext| ext.to_str()) {
        Some("sam" | "bam") => FileFormat::Alignment,
        Some("fastq" | "fq" | "fasta" | "fa") => FileFormat::Fastx,
        Some("gz") => {
            let p = args.input.with_extension("");
            let ext = p.extension().and_then(|ext| ext.to_str());
            match ext {
                Some("sam" | "bam") => FileFormat::Alignment,
                Some("fastq" | "fq" | "fasta" | "fa") => FileFormat::Fastx,
                _ => return Err(anyhow!("Unrecognized file extension for input file")),
            }
        }
        _ => return Err(anyhow!("Unrecognized file extension for input file")),
    };

    let output_type = match &args.output {
        None => input_format,
        Some(p) => match &p.extension().and_then(|ext| ext.to_str()) {
            Some("sam" | "bam") => FileFormat::Alignment,
            Some("fastq" | "fq" | "fasta" | "fa") => FileFormat::Fastx,
            Some("gz") => {
                let p = p.with_extension("");
                let ext = p.extension().and_then(|ext| ext.to_str());
                match ext {
                    Some("sam" | "bam") => FileFormat::Alignment,
                    Some("fastq" | "fq" | "fasta" | "fa") => FileFormat::Fastx,
                    _ => return Err(anyhow!("Unrecognized file extension for output file")),
                }
            }
            _ => return Err(anyhow!("Unrecognized file extension for output file")),
        },
    };
    if input_format != output_type {
        return Err(anyhow!("Input and output file formats do not match"));
    }

    let input_fastx = Fastx::from_path(&args.input);
    let mut bam_reader =
        noodles_util::alignment::io::reader::Builder::default().build_from_path(&args.input)?;

    info!("Extracting read start times...");

    let start_times = match input_format {
        FileFormat::Fastx => input_fastx.start_times(),
        FileFormat::Alignment => bam_reader.start_times(),
    }
    .context("Failed to extract start times")?;

    if start_times.is_empty() {
        return Err(anyhow!("Did not find any start times in the input"));
    }

    info!("Gathered start times for {} reads", start_times.len());

    // safe to unwrap as we know start times is not empty
    let (first_timestamp, last_timestamp) = match start_times.iter().minmax() {
        NoElements => return Err(anyhow!("No start times in input fastq")),
        OneElement(el) => (*el, *el),
        MinMax(x, y) => (*x, *y),
    };

    if args.show {
        println!("Earliest: {}", first_timestamp.format(TIME_FMT)?);
        println!("Latest  : {}", last_timestamp.format(TIME_FMT)?);
        return Ok(());
    }
    info!(
        "First and last timestamps in the input are {} and {}",
        first_timestamp.format(TIME_FMT)?,
        last_timestamp.format(TIME_FMT)?
    );

    let earliest = match args.earliest {
        None => first_timestamp.to_owned(),
        Some(s) => match PrimitiveDateTime::parse(&s, &Rfc3339) {
            Ok(t) => t,
            Err(_) => {
                let duration = Duration::from_str(&s)?;
                if duration.is_negative() {
                    last_timestamp
                        .checked_add(duration)
                        .context("Subtracting --from from the last timestamp caused an overflow")?
                } else {
                    first_timestamp
                        .checked_add(duration)
                        .context("Adding --from to the first timestamp caused an overflow")?
                }
            }
        },
    };

    let latest = match args.latest {
        None => last_timestamp.to_owned(),
        Some(s) => match PrimitiveDateTime::parse(&s, &Rfc3339) {
            Ok(t) => t,
            Err(_) => {
                let duration = Duration::from_str(&s)?;
                if duration.is_negative() {
                    last_timestamp
                        .checked_add(duration)
                        .context("Subtracting --to from the last timestamp caused an overflow")?
                } else {
                    first_timestamp
                        .checked_add(duration)
                        .context("Adding --to to the first timestamp caused an overflow")?
                }
            }
        },
    };

    if latest < earliest {
        return Err(anyhow!(
            "The earliest timestamp is after the latest timestamp"
        ));
    }

    info!(
        "Extracting reads with a start time between {} and {}...",
        earliest, latest
    );
    let (reads_to_keep, nb_reads_to_keep) = valid_indices(&start_times, &earliest, &latest);

    match output_type {
        FileFormat::Fastx => {
            let mut output_handle = match &args.output {
                None => match args.output_type {
                    None => Box::new(stdout()),
                    Some(fmt) => {
                        niffler::basic::get_writer(Box::new(stdout()), fmt, args.compress_level)?
                    }
                },
                Some(p) => {
                    let out_fastx = Fastx::from_path(p);
                    out_fastx
                        .create(args.compress_level, args.output_type)
                        .context("Failed to create the output file")?
                }
            };

            input_fastx.extract_reads_in_timeframe_into(
                &reads_to_keep,
                nb_reads_to_keep,
                &mut output_handle,
            )?;
        }
        FileFormat::Alignment => {
            let mut writer = match &args.output {
                None => noodles_util::alignment::io::writer::Builder::default()
                    .build_from_writer(Box::new(stdout()))?,
                Some(p) => {
                    noodles_util::alignment::io::writer::Builder::default().build_from_path(p)?
                }
            };

            let mut bam_reader = noodles_util::alignment::io::reader::Builder::default()
                .build_from_path(&args.input)?;
            let header = bam_reader.read_header()?;
            writer.write_header(&header)?;
            // need to reopen the bam reader as the header has been read and we need to read it again
            let mut bam_reader = noodles_util::alignment::io::reader::Builder::default()
                .build_from_path(&args.input)?;
            bam_reader.extract_reads_in_timeframe_into(
                &reads_to_keep,
                nb_reads_to_keep,
                &mut writer,
            )?;
            writer.finish(&header)?;
        }
    };

    info!("Done! Kept {} reads", nb_reads_to_keep);

    Ok(())
}
