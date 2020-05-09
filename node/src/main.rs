//! Node CLI
#![allow(clippy::too_many_arguments)]
#![allow(clippy::clone_double_ref)]
#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;
mod cli;
mod command;

fn main() -> sc_cli::Result<()> {
    command::run()
}
