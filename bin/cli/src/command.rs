use bounty_cli::{
    bank,
    bounty,
    donate,
    key,
    org,
    shares,
    vote,
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
    Vote(VoteCommand),
    Donate(DonateCommand),
    Bank(BankCommand),
    Bounty(BountyCommand),
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

#[derive(Clone, Debug, Clap)]
pub struct VoteCommand {
    #[clap(subcommand)]
    pub cmd: VoteSubCommand,
}

#[derive(Clone, Debug, Clap)]
pub enum VoteSubCommand {
    CreateThresholdApprovalVote(vote::VoteCreateThresholdApprovalCommand),
    CreateUnanimousConsentVote(vote::VoteCreateUnanimousConsentCommand),
    SubmitVote(vote::VoteSubmitCommand),
}

#[derive(Clone, Debug, Clap)]
pub struct DonateCommand {
    #[clap(subcommand)]
    pub cmd: DonateSubCommand,
}

#[derive(Clone, Debug, Clap)]
pub enum DonateSubCommand {
    DonateWithFee(donate::DonateWithFeeCommand),
    DonateWithoutFee(donate::DonateWithoutFeeCommand),
}

#[derive(Clone, Debug, Clap)]
pub struct BankCommand {
    #[clap(subcommand)]
    pub cmd: BankSubCommand,
}

#[derive(Clone, Debug, Clap)]
pub enum BankSubCommand {
    OpenAccount(bank::BankOpenOrgAccountCommand),
    OpenAccount2(bank::BankOpenOrgAccountCommand),
}

#[derive(Clone, Debug, Clap)]
pub struct BountyCommand {
    #[clap(subcommand)]
    pub cmd: BountySubCommand,
}

#[derive(Clone, Debug, Clap)]
pub enum BountySubCommand {
    PostBounty(bounty::BountyPostCommand),
    ApplyForBounty(bounty::BountyApplicationCommand),
    TriggerApplicationReview(bounty::BountyTriggerApplicationReviewCommand),
    SudoApproveApplication(bounty::BountySudoApproveApplicationCommand),
    PollApplication(bounty::BountyPollApplicationCommand),
    SubmitMilestone(bounty::BountySubmitMilestoneCommand),
    TriggerMilestoneReview(bounty::BountyTriggerMilestoneReviewCommand),
    SudoApproveMilestone(bounty::BountySudoApproveMilestoneCommand),
    PollMilestone(bounty::BountyPollMilestoneCommand),
}
