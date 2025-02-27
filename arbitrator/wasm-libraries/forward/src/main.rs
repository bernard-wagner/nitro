// Copyright 2022-2024, Offchain Labs, Inc.
// For license information, see https://github.com/OffchainLabs/nitro/blob/master/LICENSE.md

use eyre::Result;
use std::{fs::File, path::PathBuf};
use structopt::StructOpt;

use forward::{forward, forward_stub};

#[derive(StructOpt)]
#[structopt(name = "arbitrator-prover")]
struct Opts {
    #[structopt(long)]
    path: PathBuf,
    #[structopt(long)]
    stub: bool,
}

fn main() -> Result<()> {
    let opts = Opts::from_args();
    let file = &mut File::options()
        .create(true)
        .write(true)
        .truncate(true)
        .open(opts.path)?;

    match opts.stub {
        true => forward_stub(file),
        false => forward(file),
    }
}