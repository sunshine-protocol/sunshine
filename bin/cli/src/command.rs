use bounty_cli::{
    key,
    org,
    shares,
    wallet,
};
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
    Wallet(WalletCommand),
    Org(OrgCommand),
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

#[derive(Clone, Debug, Clap)]
pub struct WalletCommand {
    #[clap(subcommand)]
    pub cmd: WalletSubCommand,
}

#[derive(Clone, Debug, Clap)]
pub enum WalletSubCommand {
    GetAccountBalance(wallet::WalletBalanceCommand),
    TransferBalance(wallet::WalletTransferCommand),
}

#[derive(Clone, Debug, Clap)]
pub struct OrgCommand {
    #[clap(subcommand)]
    pub cmd: OrgSubCommand,
}

#[derive(Clone, Debug, Clap)]
pub enum OrgSubCommand {
    // share stuff
    IssueShares(shares::SharesIssueCommand),
    BurnShares(shares::SharesBurnCommand),
    BatchIssueShares(shares::SharesBatchIssueCommand),
    BatchBurnShares(shares::SharesBatchBurnCommand),
    ReserveShares(shares::SharesReserveCommand),
    UnreserveShares(shares::SharesUnReserveCommand),
    LockShares(shares::SharesLockCommand),
    UnlockShares(shares::SharesUnLockCommand),
    // full org stuff
    RegisterFlatOrg(org::OrgRegisterFlatCommand),
    RegisterWeightedOrg(org::OrgRegisterWeightedCommand),
}
