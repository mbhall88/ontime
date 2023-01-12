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
use ontime::DurationExt;
use std::io::stdout;
use time::format_description::well_known::Iso8601;
use time::{Duration, PrimitiveDateTime};

fn main() -> Result<()> {
    let args = Cli::parse();
    // setup logging
    let mut log_builder = Builder::new();
    log_builder
        .filter(None, LevelFilter::Info)
        .format_module_path(false)
        .format_target(false)
        .init();
    info!("{:?}", args);

    let input_fastx = Fastx::from_path(&args.input);

    let mut _output_handle = match args.output {
        None => match args.output_type {
            None => Box::new(stdout()),
            Some(fmt) => niffler::basic::get_writer(Box::new(stdout()), fmt, args.compress_level)?,
        },
        Some(p) => {
            let out_fastx = Fastx::from_path(&p);
            out_fastx
                .create(args.compress_level, args.output_type)
                .context("unable to create the output file")?
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

    info!(
        "First and last timestamps in the input are {} and {}",
        first_timestamp, last_timestamp
    );

    let earliest = match args.earliest {
        None => first_timestamp.to_owned(),
        Some(s) => match PrimitiveDateTime::parse(&s, &Iso8601::DEFAULT) {
            Ok(t) => t,
            Err(_) => {
                let duration = Duration::from_str(&s)?;
                if duration.is_negative() {
                    last_timestamp.checked_add(duration).context(
                        "Subtracting --earliest from the last timestamp caused an overflow",
                    )?
                } else {
                    first_timestamp
                        .checked_add(duration)
                        .context("Adding --earliest to the first timestamp caused an overflow")?
                }
            }
        },
    };

    let latest = match args.latest {
        None => last_timestamp.to_owned(),
        Some(s) => match PrimitiveDateTime::parse(&s, &Iso8601::DEFAULT) {
            Ok(t) => t,
            Err(_) => {
                let duration = Duration::from_str(&s)?;
                if duration.is_negative() {
                    last_timestamp.checked_add(duration).context(
                        "Subtracting --latest from the last timestamp caused an overflow",
                    )?
                } else {
                    first_timestamp
                        .checked_add(duration)
                        .context("Adding --latest to the first timestamp caused an overflow")?
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
        "Extracting reads with a start time between {} and {}",
        earliest, latest
    );

    Ok(())
}
