mod cli;
mod io;

use crate::cli::Cli;
use crate::io::Fastx;
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use env_logger::Builder;
use log::info;
use log::LevelFilter;
use std::io::stdout;
use time::format_description::well_known::Iso8601;
use time::PrimitiveDateTime;

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
    let first_timestamp = start_times.iter().min().unwrap();

    let earliest = match args.earliest {
        None => first_timestamp.to_owned(),
        Some(s) => match PrimitiveDateTime::parse(&s, &Iso8601::DEFAULT) {
            Ok(t) => t,
            Err(_) => todo!(),
        },
    };

    Ok(())
}
