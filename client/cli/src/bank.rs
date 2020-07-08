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
use bounty_client::Account;
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
pub struct BankOpenOrgAccountCommand {
    pub seed: u128,
    pub hosting_org: u64,
    pub bank_operator: Option<String>,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for BankOpenOrgAccountCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Bank>::Currency: From<u128> + Display,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let bank_operator: Option<T::AccountId> =
            if let Some(acc) = &self.bank_operator {
                let new_acc: Account<T> = acc.parse()?;
                Some(new_acc.id)
            } else {
                None
            };
        let event = client
            .open_org_bank_account(
                self.seed.into(),
                self.hosting_org.into(),
                bank_operator,
            )
            .await?;
        println!(
            "Account {} initialized new bank account {:?} with balance {} for Org {} with bank operator {:?}",
            event.seeder, event.new_bank_id, event.seed, event.hosting_org, event.bank_operator
        );
        Ok(())
    }
}
