use clap::Clap;
use std::path::PathBuf;
use sunshine_bounty_cli::{
    bank,
    bounty,
    donate,
    org,
    shares,
    vote,
};
use sunshine_cli_utils::{
    key,
    wallet,
};

#[derive(Clone, Debug, Clap)]
pub struct Opts {
    #[clap(subcommand)]
    pub cmd: SubCommand,
    #[clap(short = 'p', long = "path")]
    pub path: Option<PathBuf>,
    #[clap(short = 'c', long = "chain-spec-path")]
    pub chain_spec_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Clap)]
pub enum SubCommand {
    Key(KeyCommand),
    Wallet(WalletCommand),
    Org(OrgCommand),
    Vote(VoteCommand),
    Donate(DonateCommand),
    Bank(BankCommand),
    Bounty(BountyCommand),
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
    // full org stuff
    RegisterFlatOrg(org::NewFlatOrgCommand),
    RegisterWeightedOrg(org::NewWeightedOrgCommand),
}

#[derive(Clone, Debug, Clap)]
pub struct VoteCommand {
    #[clap(subcommand)]
    pub cmd: VoteSubCommand,
}

#[derive(Clone, Debug, Clap)]
pub enum VoteSubCommand {
    CreateSignalThresholdVote(vote::VoteCreateSignalThresholdCommand),
    CreatePercentThresholdVote(vote::VoteCreatePercentThresholdCommand),
    SubmitVote(vote::VoteSubmitCommand),
}

#[derive(Clone, Debug, Clap)]
pub struct DonateCommand {
    #[clap(subcommand)]
    pub cmd: DonateSubCommand,
}

#[derive(Clone, Debug, Clap)]
pub enum DonateSubCommand {
    PropDonate(donate::PropDonateCommand),
    EqualDonate(donate::EqualDonateCommand),
}

#[derive(Clone, Debug, Clap)]
pub struct BankCommand {
    #[clap(subcommand)]
    pub cmd: BankSubCommand,
}

#[derive(Clone, Debug, Clap)]
pub enum BankSubCommand {
    Open(bank::OpenCommand),
    ProposeSpend(bank::ProposeSpendCommand),
    TriggerVote(bank::TriggerVoteCommand),
    SudoApprove(bank::SudoApproveCommand),
    Close(bank::CloseCommand),
}

#[derive(Clone, Debug, Clap)]
pub struct BountyCommand {
    #[clap(subcommand)]
    pub cmd: BountySubCommand,
}

#[derive(Clone, Debug, Clap)]
pub enum BountySubCommand {
    PostBounty(bounty::BountyPostCommand),
    ContributeToBounty(bounty::BountyContributeCommand),
    SubmitForBounty(bounty::BountySubmitCommand),
    ApproveApplication(bounty::BountyApproveCommand),
    // storage helpers
    GetBounty(bounty::GetBountyCommand),
    GetSubmission(bounty::GetSubmissionCommand),
    GetOpenBounties(bounty::GetOpenBountiesCommand),
    GetOpenSubmissions(bounty::GetOpenSubmissionsCommand),
}
