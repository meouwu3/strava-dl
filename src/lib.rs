mod archive;
mod cli;
mod client;
mod discovery;
mod extract;
mod hls;
mod model;
mod progress;

use anyhow::{anyhow, Result};
use clap::Parser;

use archive::write_archive;
use client::StravaClient;
use discovery::{discover, DiscoverOpts};
use extract::{assign_flat_names, is_athlete_id};
use model::ArchivePlan;

enum Stage {
    Scan,
    Download,
}

pub fn run() {
    match run_inner() {
        Ok(()) => {}
        Err((Stage::Scan, err)) => {
            progress::scan_failed(&format!("{err:#}"));
            std::process::exit(1);
        }
        Err((Stage::Download, err)) => {
            progress::download_failed(&format!("{err:#}"));
            std::process::exit(1);
        }
    }
}

fn run_inner() -> Result<(), (Stage, anyhow::Error)> {
    let args = cli::Args::parse();
    if !is_athlete_id(&args.athlete_id) {
        return Err((Stage::Scan, anyhow!("athlete id must contain only digits")));
    }
    let authenticated = args.cookie.is_some() || args.token.is_some();
    let client = StravaClient::new(args.timeout, args.cookie, args.token, args.csrf)
        .map_err(|err| (Stage::Scan, err))?;
    let opts = DiscoverOpts {
        base_url: args.base_url.trim_end_matches('/'),
        athlete_id: &args.athlete_id,
        max_items: None,
    };
    let mut items = discover(&client, &opts).map_err(|err| (Stage::Scan, err))?;
    assign_flat_names(&args.athlete_id, &mut items);
    if items.is_empty() {
        return Err((
            Stage::Scan,
            anyhow!("no accessible media found; Strava privacy or login may be required"),
        ));
    }
    if !authenticated {
        eprintln!("warning: no --cookie or --token; athlete photos usually require login");
    }
    let count = items.len();
    let plan = ArchivePlan {
        athlete_id: args.athlete_id,
        output: args.output.into(),
        output_dir: args.output_dir,
        items,
    };
    write_archive(&client, &plan).map_err(|err| (Stage::Download, err))?;
    progress::complete(count);
    Ok(())
}
