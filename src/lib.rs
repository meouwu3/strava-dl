mod archive;
mod cli;
mod client;
mod discovery;
mod extract;
mod hls;
mod model;

use anyhow::{bail, Result};
use clap::Parser;

use archive::write_archive;
use client::StravaClient;
use discovery::{current_year_month, discover, DiscoverOpts};
use extract::is_athlete_id;
use model::ArchivePlan;

pub fn run() -> Result<()> {
    let args = cli::Args::parse();
    if !is_athlete_id(&args.athlete_id) {
        bail!("athlete id must contain only digits");
    }
    let authenticated = args.cookie.is_some() || args.token.is_some();
    let client = StravaClient::new(args.timeout, args.cookie, args.token, args.csrf)?;
    let (year, month) = current_year_month();
    let opts = DiscoverOpts {
        base_url: args.base_url.trim_end_matches('/'),
        athlete_id: &args.athlete_id,
        authenticated,
        year,
        month,
        max_activities: None,
    };
    let items = discover(&client, &opts)?;
    if items.is_empty() {
        bail!("no accessible media found; Strava privacy or login may be required");
    }
    if !authenticated {
        eprintln!(
            "warning: no --cookie or --token; only publicly visible profile media was requested"
        );
    }
    let count = items.len();
    let plan = ArchivePlan {
        athlete_id: args.athlete_id,
        output: args.output.into(),
        output_dir: args.output_dir,
        items,
    };
    let dest = write_archive(&client, &plan)?;
    println!("{}", dest.display());
    eprintln!("archived {count} media files");
    Ok(())
}
