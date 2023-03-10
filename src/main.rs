mod cli;
mod io;

use crate::cli::Cli;
use crate::io::Fastx;
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

fn main() -> Result<()> {
    let args = Cli::parse();
    // setup logging
    let mut log_builder = Builder::new();
    log_builder
        .filter(None, LevelFilter::Info)
        .format_module_path(false)
        .format_target(false)
        .init();

    let input_fastx = Fastx::from_path(&args.input);

    let mut output_handle = match args.output {
        None => match args.output_type {
            None => Box::new(stdout()),
            Some(fmt) => niffler::basic::get_writer(Box::new(stdout()), fmt, args.compress_level)?,
        },
        Some(p) => {
            let out_fastx = Fastx::from_path(&p);
            out_fastx
                .create(args.compress_level, args.output_type)
                .context("Failed to create the output file")?
        }
    };

    info!("Extracting read start times...");

    let start_times = input_fastx
        .start_times()
        .context("Failed to parse a start time")?;

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
    } else {
        info!(
            "First and last timestamps in the input are {} and {}",
            first_timestamp.format(TIME_FMT)?,
            last_timestamp.format(TIME_FMT)?
        );
    }

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
    input_fastx.extract_reads_in_timeframe_into(
        &reads_to_keep,
        nb_reads_to_keep,
        &mut output_handle,
    )?;

    info!("Done! Kept {} reads", nb_reads_to_keep);

    Ok(())
}
