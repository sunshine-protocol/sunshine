use sc_cli::{
    RunCmd,
    Runner,
    RuntimeVersion,
    SubstrateCli,
};
use sc_service::{
    ChainSpec,
    DatabaseConfig,
    Role,
};
use structopt::StructOpt;
use tiny_multihash::Multihash;

#[derive(Debug, StructOpt)]
pub struct Cli {
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,

    #[structopt(flatten)]
    pub run: RunCmd,
}

#[derive(Debug, StructOpt)]
pub enum Subcommand {
    PurgeChain(sc_cli::PurgeChainCmd),
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
            "dev" => Box::new(test_node::development_config()),
            "" | "local" => Box::new(test_node::local_testnet_config()),
            path => {
                Box::new(test_node::ChainSpec::from_json_file(path.into())?)
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
        Some(Subcommand::PurgeChain(cmd)) => {
            let mut runner = cli.create_runner(cmd)?;
            force_parity_db(&mut runner);
            runner.sync_run(|config| cmd.run(config.database))
        }
        None => {
            let mut runner = cli.create_runner(&cli.run)?;
            force_parity_db(&mut runner);
            runner.run_node_until_exit(|config| {
                match config.role {
                    Role::Light => test_node::new_light::<Multihash>(config),
                    _ => test_node::new_full::<Multihash>(config),
                }
                .map(|service| service.0)
            })
        }
    }
}

fn force_parity_db(runner: &mut Runner<Cli>) {
    let config = runner.config_mut();
    let path = config.database.path().unwrap().to_path_buf();
    config.database = DatabaseConfig::ParityDb { path };
}
