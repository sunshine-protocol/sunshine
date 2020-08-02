use crate::error::{
    Error,
    Result,
};
use clap::Clap;
use core::fmt::{
    Debug,
    Display,
};
use substrate_subxt::{
    sp_core::crypto::Ss58Codec,
    system::System,
    Runtime,
};
use sunshine_bounty_client::{
    donate::{
        Donate,
        DonateClient,
    },
    org::Org,
};
use sunshine_core::Ss58;

#[derive(Clone, Debug, Clap)]
pub struct PropDonateCommand {
    pub org: u64,
    pub rem_recipient: String,
    pub amt: u128,
}

impl PropDonateCommand {
    pub async fn exec<R: Runtime + Donate, C: DonateClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Org>::OrgId: From<u64> + Display,
        <R as Donate>::DCurrency: From<u128> + Display,
    {
        let remainder_recipient: Ss58<R> = self.rem_recipient.parse()?;
        let event = client
            .make_prop_donation(
                self.org.into(),
                remainder_recipient.0,
                self.amt.into(),
            )
            .await
            .map_err(Error::Client)?;
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
    pub async fn exec<R: Runtime + Donate, C: DonateClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Org>::OrgId: From<u64> + Display,
        <R as Donate>::DCurrency: From<u128> + Display,
    {
        let remainder_recipient: Ss58<R> = self.rem_recipient.parse()?;
        let event = client
            .make_equal_donation(
                self.org.into(),
                remainder_recipient.0,
                self.amt.into(),
            )
            .await
            .map_err(Error::Client)?;
        println!(
            "AccountId {:?} donated {} to flat OrgId {} and {} to the Remainder Recipient {}",
            event.sender, event.amt_to_org, event.org, event.amt_to_recipient, event.rem_recipient,
        );
        Ok(())
    }
}
