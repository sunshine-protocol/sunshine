use bounty_cli::key;
use clap::Clap;
use std::path::PathBuf;

#[derive(Clone, Debug, Clap)]
pub struct Opts {
    #[clap(subcommand)]
    pub cmd: SubCommand,
    #[clap(short = "p", long = "path")]
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, Clap)]
pub enum SubCommand {
    Key(KeyCommand),
    // Wallet(WalletCommand),
    Run,
}

#[derive(Clone, Debug, Clap)]
pub struct KeyCommand {
    #[clap(subcommand)]
    pub cmd: KeySubCommand,
}

#[derive(Clone, Debug, Clap)]
pub enum KeySubCommand {
    Set(key::KeySetCommand),
    Unlock(key::KeyUnlockCommand),
    Lock(key::KeyLockCommand),
}

// #[derive(Clone, Debug, Clap)]
// pub struct WalletCommand {
//     #[clap(subcommand)]
//     pub cmd: WalletSubCommand,
// }

// #[derive(Clone, Debug, Clap)]
// pub enum WalletSubCommand {
//     IssueShares(WalletIssueSharesCommand),
//     ReserveShares(WalletReserveSharesCommand),
//     LockShares(WalletLockSharesCommand),
// }
