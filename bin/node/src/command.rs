use crate::{
    chain_spec,
    cli::Cli,
    service,
};
use sc_cli::{
    ChainSpec,
    Role,
    RuntimeVersion,
    SubstrateCli,
};
use sc_service::ServiceParams;

impl SubstrateCli for Cli {
    fn impl_name() -> String {
        crate::IMPL_NAME.to_string()
    }

    fn impl_version() -> String {
        crate::IMPL_VERSION.to_string()
    }

    fn description() -> String {
        crate::DESCRIPTION.to_string()
    }

    fn author() -> String {
        crate::AUTHOR.to_string()
    }

    fn support_url() -> String {
        crate::SUPPORT_URL.to_string()
    }

    fn copyright_start_year() -> i32 {
        crate::COPYRIGHT_START_YEAR
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
        &suntime::VERSION
    }
}

/// Parse and run command line arguments
pub fn run() -> sc_cli::Result<()> {
    let cli = Cli::from_args();

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
