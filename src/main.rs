mod chain_spec;
mod rpc;
#[macro_use]
mod service;
mod cli;
mod command;

pub use sc_cli::{error, VersionInfo};

fn main() -> Result<(), error::Error> {
    let version = VersionInfo {
        name: "SunshineChain",
        commit: env!("VERGEN_SHA_SHORT"),
        version: env!("CARGO_PKG_VERSION"),
        executable_name: "sunshine",
        author: "4meta5",
        description: "sunshine-chain",
        support_url: "https://github.com/web3garden/sunshine",
        copyright_start_year: 2020,
    };

    command::run(version)
}
