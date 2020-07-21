use clap::Clap;
use std::path::PathBuf;
use sunshine_bounty_cli::{
    bounty,
    org,
    shares,
    vote,
};
use sunshine_identity_cli::key;

#[derive(Clone, Debug, Clap)]
pub struct PathOpts {
    #[clap(short = "p", long = "path")]
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, Clap)]
pub struct CommandOpts {
    #[clap(subcommand)]
    pub cmd: SubCommand,
}

#[derive(Clone, Debug, Clap)]
pub enum SubCommand {
    Key(KeyCommand),
    Org(OrgCommand),
    Vote(VoteCommand),
    Bounty(BountyCommand),
    Exit,
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
    CreateSignalThresholdVote(vote::VoteCreateSignalThresholdCommand),
    CreatePercentThresholdVote(vote::VoteCreatePercentThresholdCommand),
    CreateUnanimousConsentVote(vote::VoteCreateUnanimousConsentCommand),
    SubmitVote(vote::VoteSubmitCommand),
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
