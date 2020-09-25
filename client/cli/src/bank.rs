#![allow(clippy::type_complexity)]
use crate::{
    error::VotePercentThresholdInputBoundError,
    vote::*,
};
use clap::Clap;
use core::fmt::{
    Debug,
    Display,
};
use substrate_subxt::{
    balances::Balances,
    sp_core::crypto::Ss58Codec,
    sp_runtime::Permill,
    system::System,
};
use sunshine_bounty_client::{
    bank::{
        Bank,
        BankClient,
    },
    org::Org,
    vote::Vote,
};
use sunshine_bounty_utils::{
    organization::OrgRep,
    vote::{
        Threshold,
        ThresholdInput,
        XorThreshold,
    },
};
use sunshine_client_utils::{
    crypto::ss58::Ss58,
    Node,
    Result,
};

#[derive(Clone, Debug, Clap)]
pub struct OpenCommand {
    pub seed: u128,
    pub hosting_org: u64,
    pub bank_operator: Option<String>,
    pub percent_threshold: u8,
}

impl OpenCommand {
    pub async fn exec<N: Node, C: BankClient<N>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        N::Runtime: Bank,
        <N::Runtime as System>::AccountId: Ss58Codec,
        <N::Runtime as Org>::OrgId: From<u64> + Display,
        <N::Runtime as Vote>::Percent: From<Permill>,
        <N::Runtime as Balances>::Balance: From<u128> + Display,
    {
        let bank_operator = if let Some(acc) = &self.bank_operator {
            let new_acc: Ss58<N::Runtime> = acc.parse()?;
            Some(new_acc.0)
        } else {
            None
        };
        let support: <N::Runtime as Vote>::Percent =
            u8_to_permill(self.percent_threshold)
                .map_err(|_| VotePercentThresholdInputBoundError)?
                .into();
        let threshold: Threshold<<N::Runtime as Vote>::Percent> =
            Threshold::new(support, None);
        let threshold_config: ThresholdInput<
            OrgRep<<N::Runtime as Org>::OrgId>,
            XorThreshold<<N::Runtime as Vote>::Signal, <N::Runtime as Vote>::Percent>,
        > = ThresholdInput::new(
            OrgRep::Equal(self.hosting_org.into()),
            XorThreshold::Percent(threshold),
        );
        let event = client
            .open(
                self.seed.into(),
                self.hosting_org.into(),
                bank_operator,
                threshold_config,
            )
            .await?;
        println!(
            "Account {} initialized new bank account {:?} with balance {} for Org {} with bank operator {:?}",
            event.seeder, event.new_bank_id, event.seed, event.hosting_org, event.bank_operator
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct ProposeSpendCommand {
    pub bank_id: u64,
    pub amount: u128,
    pub dest: String,
}

impl ProposeSpendCommand {
    pub async fn exec<N: Node, C: BankClient<N>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        N::Runtime: Bank,
        <N::Runtime as System>::AccountId: Ss58Codec,
        <N::Runtime as Bank>::BankId: From<u64> + Display,
        <N::Runtime as Balances>::Balance: From<u128> + Display,
    {
        let raw_dest: Ss58<N::Runtime> = self.dest.parse()?;
        let event = client
            .propose_spend(self.bank_id.into(), self.amount.into(), raw_dest.0)
            .await?;
        println!(
            "Account {} proposed new spend from Bank {:?} with Spend Proposal ID {:?} of Amount {} to Destination {:?}",
            event.caller, event.bank_id, event.spend_id, event.amount, event.dest
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct TriggerVoteCommand {
    pub bank_id: u64,
    pub spend_id: u64,
}

impl TriggerVoteCommand {
    pub async fn exec<N: Node, C: BankClient<N>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        N::Runtime: Bank,
        <N::Runtime as System>::AccountId: Ss58Codec,
        <N::Runtime as Bank>::BankId: From<u64> + Display,
        <N::Runtime as Bank>::SpendId: From<u64> + Display,
        <N::Runtime as Vote>::VoteId: Display,
    {
        let event = client
            .trigger_vote(self.bank_id.into(), self.spend_id.into())
            .await?;
        println!(
            "Account {} triggered VoteID {} for Bank {:?} Spend Proposal {:?}",
            event.caller, event.vote_id, event.bank_id, event.spend_id
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct SudoApproveCommand {
    pub bank_id: u64,
    pub spend_id: u64,
}

impl SudoApproveCommand {
    pub async fn exec<N: Node, C: BankClient<N>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        N::Runtime: Bank,
        <N::Runtime as System>::AccountId: Ss58Codec,
        <N::Runtime as Bank>::BankId: From<u64> + Display,
        <N::Runtime as Bank>::SpendId: From<u64> + Display,
        <N::Runtime as Vote>::VoteId: Display,
    {
        let event = client
            .sudo_approve(self.bank_id.into(), self.spend_id.into())
            .await?;
        println!(
            "Account {} sudo approved Bank {:?} Spend Proposal {:?}",
            event.caller, event.bank_id, event.spend_id
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct CloseCommand {
    pub bank_id: u64,
}

impl CloseCommand {
    pub async fn exec<N: Node, C: BankClient<N>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        N::Runtime: Bank,
        <N::Runtime as System>::AccountId: Ss58Codec,
        <N::Runtime as Bank>::BankId: From<u64> + Display,
        <N::Runtime as Bank>::SpendId: From<u64> + Display,
    {
        let event = client.close(self.bank_id.into()).await?;
        println!(
            "Account {} closed Bank {:?} for Org {:?}",
            event.closer, event.bank_id, event.org
        );
        Ok(())
    }
}
