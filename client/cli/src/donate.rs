use clap::Clap;
use core::fmt::{
    Debug,
    Display,
};
use substrate_subxt::{
    balances::Balances,
    sp_core::crypto::Ss58Codec,
    system::System,
};
use sunshine_bounty_client::{
    donate::{
        Donate,
        DonateClient,
    },
    org::Org,
};
use sunshine_client_utils::{
    crypto::ss58::Ss58,
    Node,
    Result,
};

#[derive(Clone, Debug, Clap)]
pub struct PropDonateCommand {
    pub org: u64,
    pub rem_recipient: String,
    pub amt: u128,
}

impl PropDonateCommand {
    pub async fn exec<N: Node, C: DonateClient<N>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        N::Runtime: Donate,
        <N::Runtime as System>::AccountId: Ss58Codec,
        <N::Runtime as Org>::OrgId: From<u64> + Display,
        <N::Runtime as Balances>::Balance: From<u128> + Display,
    {
        let remainder_recipient: Ss58<N::Runtime> = self.rem_recipient.parse()?;
        let event = client
            .make_prop_donation(
                self.org.into(),
                remainder_recipient.0,
                self.amt.into(),
            )
            .await?;
        println!(
            "AccountId {:?} donated {} to weighted OrgId {} and {} to the Remainder Recipient {}",
            event.sender, event.amt_to_org, event.org, event.amt_to_recipient, event.rem_recipient,
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct EqualDonateCommand {
    pub org: u64,
    pub rem_recipient: String,
    pub amt: u128,
}

impl EqualDonateCommand {
    pub async fn exec<N: Node, C: DonateClient<N>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        N::Runtime: Donate,
        <N::Runtime as System>::AccountId: Ss58Codec,
        <N::Runtime as Org>::OrgId: From<u64> + Display,
        <N::Runtime as Balances>::Balance: From<u128> + Display,
    {
        let remainder_recipient: Ss58<N::Runtime> = self.rem_recipient.parse()?;
        let event = client
            .make_equal_donation(
                self.org.into(),
                remainder_recipient.0,
                self.amt.into(),
            )
            .await?;
        println!(
            "AccountId {:?} donated {} to flat OrgId {} and {} to the Remainder Recipient {}",
            event.sender, event.amt_to_org, event.org, event.amt_to_recipient, event.rem_recipient,
        );
        Ok(())
    }
}
