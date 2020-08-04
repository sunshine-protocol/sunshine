//! Substrate Node Template CLI library.

use sc_cli::{
    RunCmd,
    RuntimeVersion,
    Subcommand,
    SubstrateCli,
};
use sc_service::{
    ChainSpec,
    Role,
    ServiceParams,
};
use structopt::StructOpt;
use test_node::{
    chain_spec,
    service,
};

#[derive(Debug, StructOpt)]
pub struct Cli {
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[structopt(flatten)]
    pub run: RunCmd,
}

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        test_node::IMPL_NAME.into()
    }

    fn impl_version() -> String {
        test_node::IMPL_VERSION.into()
    }

    fn description() -> String {
        test_node::DESCRIPTION.into()
    }

    fn author() -> String {
        test_node::AUTHOR.into()
    }

    fn support_url() -> String {
        test_node::SUPPORT_URL.into()
    }

    fn copyright_start_year() -> i32 {
        test_node::COPYRIGHT_START_YEAR
    }

    fn executable_name() -> String {
        test_node::EXECUTABLE_NAME.into()
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

    fn native_runtime_version(
        _: &Box<dyn ChainSpec>,
    ) -> &'static RuntimeVersion {
        &test_runtime::VERSION
    }
}

fn main() -> sc_cli::Result<()> {
    let cli = <Cli as SubstrateCli>::from_args();

    match &cli.subcommand {
        Some(subcommand) => {
            let runner = cli.create_runner(subcommand)?;
            runner.run_subcommand(subcommand, |config| {
                let ServiceParams {
                    client,
                    backend,
                    task_manager,
                    import_queue,
                    ..
                } = service::new_full_params(config)?.0;
                Ok((client, backend, import_queue, task_manager))
            })
        }
        None => {
            let runner = cli.create_runner(&cli.run)?;
            runner.run_node_until_exit(|config| {
                match config.role {
                    Role::Light => service::new_light(config),
                    _ => service::new_full(config),
                }
                .map(|service| service.0)
            })
        }
    }
}
