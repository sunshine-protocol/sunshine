use crate::{
    async_trait,
    AbstractClient,
    Bank,
    Bounty,
    Command,
    Donate,
    Org,
    Pair,
    Result,
    Runtime,
    Vote,
};
use clap::Clap;
use core::fmt::{
    Debug,
    Display,
};
use substrate_subxt::{
    sp_core::crypto::Ss58Codec,
    system::System,
};

#[derive(Clone, Debug, Clap)]
pub struct DonateWithFeeCommand {
    pub org: u64,
    pub amt: u128,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for DonateWithFeeCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Donate>::DCurrency: From<u128> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let event = client
            .make_prop_donation_with_fee(self.org.into(), self.amt.into())
            .await?;
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

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for DonateWithoutFeeCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Donate>::DCurrency: From<u128> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let event = client
            .make_prop_donation_without_fee(self.org.into(), self.amt.into())
            .await?;
        println!(
            "AccountId {:?} donated {} to OrgId {} (without the module fee)",
            event.sender, event.amt, event.org
        );
        Ok(())
    }
}
