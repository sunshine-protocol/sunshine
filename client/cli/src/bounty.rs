use crate::utils::GithubIssueMetadata;
use clap::Clap;
use core::fmt::{
    Debug,
    Display,
};
use ipld_block_builder::{
    Cache,
    Codec,
    ReadonlyCache,
};
use std::convert::TryInto;
use substrate_subxt::{
    balances::Balances,
    sp_core::crypto::Ss58Codec,
    system::System,
    Runtime,
};
use sunshine_bounty_client::{
    bounty::{
        Bounty,
        BountyClient,
    },
    BountyBody,
};
use sunshine_client_utils::{
    cid::CidBytes,
    Result,
};

#[derive(Clone, Debug, Clap)]
pub struct BountyPostCommand {
    pub issue_url: String,
    pub amount: u128,
}

impl BountyPostCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Balances>::Balance: From<u128> + Display,
        <R as Bounty>::BountyId: Display,
        <R as Bounty>::BountyPost: From<BountyBody>,
    {
        let metadata: GithubIssueMetadata =
            self.issue_url.as_str().try_into()?;
        let bounty: <R as Bounty>::BountyPost = BountyBody {
            repo_owner: metadata.owner,
            repo_name: metadata.repo,
            issue_number: metadata.issue,
        }
        .into();
        let event = client.post_bounty(bounty, self.amount.into()).await?;
        println!(
            "Depositer with AccountId {} posted new BountyId {}, Balance {}",
            event.depositer, event.id, event.amount,
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountyContributeCommand {
    pub bounty_id: u64,
    pub amount: u128,
}

impl BountyContributeCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Balances>::Balance: From<u128> + Display,
        <R as Bounty>::BountyId: From<u64> + Display,
    {
        let event = client
            .contribute_to_bounty(self.bounty_id.into(), self.amount.into())
            .await?;
        println!(
            "AccountId {} contributed ${} to BountyId {} and the Total Balance for the Bounty is now {}",
            event.contributor, event.amount, event.bounty_id, event.total
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountySubmitCommand {
    pub issue_url: String,
    pub bounty_id: u64,
    pub amount: u128,
}

impl BountySubmitCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Balances>::Balance: From<u128> + Display,
        <R as Bounty>::BountyId: From<u64> + Display,
        <R as Bounty>::SubmissionId: Display,
        <R as Bounty>::BountySubmission: From<BountyBody>,
    {
        let metadata: GithubIssueMetadata =
            self.issue_url.as_str().try_into()?;
        let bounty: <R as Bounty>::BountySubmission = BountyBody {
            repo_owner: metadata.owner,
            repo_name: metadata.repo,
            issue_number: metadata.issue,
        }
        .into();
        let event = client
            .submit_for_bounty(
                self.bounty_id.into(),
                bounty,
                self.amount.into(),
            )
            .await?;
        println!(
            "Submitter with AccountId {} submitted for BountyId {}, requesting Balance {} with SubmissionId {:?}",
            event.submitter, event.bounty_id, event.amount, event.id,
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct BountyApproveCommand {
    pub submission_id: u64,
}

impl BountyApproveCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Balances>::Balance: From<u128> + Display,
        <R as Bounty>::SubmissionId: From<u64> + Display,
        <R as Bounty>::BountyId: Display,
    {
        let event = client
            .approve_bounty_submission(self.submission_id.into())
            .await?;
        println!(
            "Approved SubmissionId {} to transfer Balance {} to AccountId {}. Remaining Balance {} for BountyId {} ",
            event.submission_id, event.amount, event.submitter, event.new_total, event.bounty_id
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct GetBountyCommand {
    pub bounty_id: u64,
}

impl GetBountyCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Balances>::Balance: Display,
        <R as Bounty>::BountyId: Display + From<u64>,
        <R as Bounty>::IpfsReference: Debug,
    {
        let bounty_state = client.bounty(self.bounty_id.into()).await?;
        println!(
            "BOUNTY {} INFORMATION: CID: {:?} | Depositor: {} | Total Balance: {} ",
            self.bounty_id, bounty_state.info(), bounty_state.depositer(), bounty_state.total(),
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct GetSubmissionCommand {
    pub submission_id: u64,
}

impl GetSubmissionCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Balances>::Balance: Display,
        <R as Bounty>::BountyId: Display,
        <R as Bounty>::SubmissionId: Display + From<u64>,
        <R as Bounty>::IpfsReference: Debug,
    {
        let submission_state =
            client.submission(self.submission_id.into()).await?;
        println!(
            "SUBMISSION {} INFORMATION: Bounty ID: {} | CID : {:?} | Submitter: {} | Total Balance: {} ",
            self.submission_id, submission_state.bounty_id(), submission_state.submission(), submission_state.submitter(), submission_state.amount(),
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct GetOpenBountiesCommand {
    pub min: u128,
}

impl GetOpenBountiesCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        R: Bounty<IpfsReference = CidBytes>,
        C::OffchainClient: Cache<Codec, BountyBody>,
        <R as Balances>::Balance: From<u128> + Display,
        <R as Bounty>::BountyId: Display,
        <R as Bounty>::SubmissionId: Display + From<u64>,
    {
        let open_bounties = client.open_bounties(self.min.into()).await?;
        if let Some(b) = open_bounties {
            for (id, bounty) in b.into_iter() {
                let event_cid = bounty.info().to_cid()?;
                match client.offchain_client().get(&event_cid).await {
                    Ok(bounty_body) => {
                        println!(
                            "Live BountyID {} has total available balance {} at {} added by {}",
                            id,
                            bounty.total(),
                            format!(
                                "https://github.com/{}/{}/issues/{}",
                                bounty_body.repo_owner,
                                bounty_body.repo_name,
                                bounty_body.issue_number
                            ),
                            bounty.depositer().to_string()
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "Error while getting bounty {}. skipping..",
                            id
                        );
                        eprintln!("{}", e);
                        continue
                    }
                }
            }
        } else {
            println!("No open bounties above the passed input minimum balance");
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct GetOpenSubmissionsCommand {
    pub bounty_id: u64,
}

impl GetOpenSubmissionsCommand {
    pub async fn exec<R: Runtime + Bounty, C: BountyClient<R>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        R: Bounty<IpfsReference = CidBytes>,
        C::OffchainClient: Cache<Codec, BountyBody>,
        <R as Balances>::Balance: Display,
        <R as Bounty>::BountyId: From<u64> + Display,
        <R as Bounty>::SubmissionId: Display,
    {
        let open_submissions =
            client.open_submissions(self.bounty_id.into()).await?;
        if let Some(s) = open_submissions {
            for (id, sub) in s.into_iter() {
                let event_cid = sub.submission().to_cid()?;
                match client.offchain_client().get(&event_cid).await {
                    Ok(submission_body) => {
                        println!("Live SubmissionID {} requests total balance {} at {} submitted by {}",
                            id,
                            sub.amount(),
                            format!(
                                "https://github.com/{}/{}/issues/{}",
                                submission_body.repo_owner,
                                submission_body.repo_name,
                                submission_body.issue_number
                            ),
                            sub.submitter().to_string()
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "Error while getting submission {}. skipping..",
                            id
                        );
                        eprintln!("{}", e);
                        continue
                    }
                };
            }
        } else {
            println!("No open submissions for the passed in BountyID");
        }
        Ok(())
    }
}
