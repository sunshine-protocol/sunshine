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
pub struct PropDonateCommand {
    pub org: u64,
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
        let event = client
            .make_prop_donation(self.org.into(), self.amt.into())
            .await
            .map_err(Error::Client)?;
        println!(
            "AccountId {:?} donated {} to weighted OrgId {} (with the module fee)",
            event.sender, event.amt, event.org
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct EqualDonateCommand {
    pub org: u64,
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
        let event = client
            .make_equal_donation(self.org.into(), self.amt.into())
            .await
            .map_err(Error::Client)?;
        println!(
            "AccountId {:?} donated {} to flat OrgId {} (with the module fee)",
            event.sender, event.amt, event.org
        );
        Ok(())
    }
}
