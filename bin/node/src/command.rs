use crate::{
    chain_spec,
    cli::Cli,
    service,
};
use sc_cli::SubstrateCli;
use sp_consensus_aura::sr25519::AuthorityPair as AuraPair;

impl SubstrateCli for Cli {
    fn impl_name() -> &'static str {
        crate::IMPL_NAME
    }

    fn impl_version() -> &'static str {
        crate::IMPL_VERSION
    }

    fn description() -> &'static str {
        crate::DESCRIPTION
    }

    fn author() -> &'static str {
        crate::AUTHOR
    }

    fn support_url() -> &'static str {
        crate::SUPPORT_URL
    }

    fn copyright_start_year() -> i32 {
        crate::COPYRIGHT_START_YEAR
    }

    fn executable_name() -> &'static str {
        crate::EXECUTABLE_NAME
    }

    fn load_spec(
        &self,
        id: &str,
    ) -> Result<Box<dyn sc_service::ChainSpec>, String> {
        Ok(match id {
            "dev" => Box::new(chain_spec::development_config()),
            "" | "local" => Box::new(chain_spec::local_testnet_config()),
            path => {
                Box::new(chain_spec::ChainSpec::from_json_file(
                    std::path::PathBuf::from(path),
                )?)
            }
        })
    }
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
    let cli = Cli::from_args();

    match &cli.subcommand {
        Some(subcommand) => {
            let runner = cli.create_runner(subcommand)?;
            runner.run_subcommand(subcommand, |config| {
                Ok(new_full_start!(config).0)
            })
        }
        None => {
            let runner = cli.create_runner(&cli.run)?;
            runner.run_node(
                service::new_light,
                service::new_full,
                suntime::VERSION,
            )
        }
    }
}
