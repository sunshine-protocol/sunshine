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

#[derive(Clone, Debug, Clap)]
pub struct DonateWithFeeCommand {
    pub org: u64,
    pub amt: u128,
}

impl DonateWithFeeCommand {
    pub async fn exec<R: Runtime + Donate, C: DonateClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Org>::OrgId: From<u64> + Display,
        <R as Donate>::DCurrency: From<u128> + Display,
    {
        let event = client
            .make_prop_donation_with_fee(self.org.into(), self.amt.into())
            .await
            .map_err(Error::Client)?;
        println!(
            "AccountId {:?} donated {} to OrgId {} (with the module fee)",
            event.sender, event.amt, event.org
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct DonateWithoutFeeCommand {
    pub org: u64,
    pub amt: u128,
}

impl DonateWithoutFeeCommand {
    pub async fn exec<R: Runtime + Donate, C: DonateClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Org>::OrgId: From<u64> + Display,
        <R as Donate>::DCurrency: From<u128> + Display,
    {
        let event = client
            .make_prop_donation_without_fee(self.org.into(), self.amt.into())
            .await
            .map_err(Error::Client)?;
        println!(
            "AccountId {:?} donated {} to OrgId {} (without the module fee)",
            event.sender, event.amt, event.org
        );
        Ok(())
    }
}
