//! Substrate Node Template CLI library.
#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;
mod cli;
mod command;

fn main() -> sc_cli::Result<()> {
    let version = sc_cli::VersionInfo {
        name: "Sunshine Node",
        commit: env!("VERGEN_SHA_SHORT"),
        version: env!("CARGO_PKG_VERSION"),
        executable_name: "sunshine-node",
        author: "4meta5",
        description: "Sunshine Node",
        support_url: "joinsunshine.com",
        copyright_start_year: 2020,
    };

    command::run(version)
}
