mod archive;
mod cli;
mod client;
mod discovery;
mod extract;
mod hls;
mod model;
mod progress;

use anyhow::{bail, Result};
use clap::Parser;

use archive::write_archive;
use client::StravaClient;
use discovery::{discover, DiscoverOpts};
use extract::{assign_flat_names, is_athlete_id};
use model::ArchivePlan;

pub fn run() {
    if run_inner().is_err() {
        progress::fail();
        std::process::exit(1);
    }
}

fn run_inner() -> Result<()> {
    let args = cli::Args::parse();
    if !is_athlete_id(&args.athlete_id) {
        bail!("athlete id must contain only digits");
    }
    let authenticated = args.cookie.is_some() || args.token.is_some();
    let client = StravaClient::new(args.timeout, args.cookie, args.token, args.csrf)?;
    let opts = DiscoverOpts {
        base_url: args.base_url.trim_end_matches('/'),
        athlete_id: &args.athlete_id,
        max_items: None,
    };
    let mut items = discover(&client, &opts)?;
    assign_flat_names(&args.athlete_id, &mut items);
    if items.is_empty() {
        bail!("no accessible media found; Strava privacy or login may be required");
    }
    if !authenticated {
        eprintln!("warning: no --cookie or --token; athlete photos usually require login");
    }
    let plan = ArchivePlan {
        athlete_id: args.athlete_id,
        output: args.output.into(),
        output_dir: args.output_dir,
        items,
    };
    write_archive(&client, &plan)?;
    progress::complete();
    Ok(())
}
