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
use sunshine_bounty_client::org::{
    AccountShare,
    Org,
    OrgClient,
};
use sunshine_core::Ss58;

#[derive(Clone, Debug, Clap)]
pub struct OrgRegisterFlatCommand {
    pub sudo: Option<String>,
    pub parent_org: Option<u64>,
    pub constitution: String,
    pub members: Vec<String>,
}

impl OrgRegisterFlatCommand {
    pub async fn exec<R: Runtime + Org, C: OrgClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Org>::OrgId: From<u64> + Display,
    {
        let sudo = if let Some(acc) = &self.sudo {
            let new_acc: Ss58<R> = acc.parse()?;
            Some(new_acc.0)
        } else {
            None
        };
        let parent_org: Option<R::OrgId> = if let Some(org) = self.parent_org {
            Some(org.into())
        } else {
            None
        };
        let members = self
            .members
            .iter()
            .map(|acc| -> Result<R::AccountId, C::Error> {
                let mem: Ss58<R> = acc.parse::<Ss58<R>>()?;
                Ok(mem.0)
            })
            .collect::<Result<Vec<R::AccountId>, C::Error>>()?;
        let event = client
            .register_flat_org(
                sudo,
                parent_org,
                (*self.constitution).to_string(),
                &members,
            )
            .await
            .map_err(Error::Client)?;
        println!(
            "Account {} created a flat organization with OrgId: {}, constitution: {:?} and {} members of equal ownership weight",
            event.caller, event.new_id, event.constitution, event.total
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct OrgRegisterWeightedCommand {
    pub sudo: Option<String>,
    pub parent_org: Option<u64>,
    pub constitution: String,
    pub members: Vec<AccountShare>,
}

impl OrgRegisterWeightedCommand {
    pub async fn exec<R: Runtime + Org, C: OrgClient<R>>(
        &self,
        client: &C,
    ) -> Result<(), C::Error>
    where
        <R as System>::AccountId: Ss58Codec,
        <R as Org>::OrgId: From<u64> + Display,
        <R as Org>::Shares: From<u64> + Display,
    {
        let sudo: Option<R::AccountId> = if let Some(acc) = &self.sudo {
            let new_acc: Ss58<R> = acc.parse::<Ss58<R>>()?;
            Some(new_acc.0)
        } else {
            None
        };
        let parent_org: Option<R::OrgId> = if let Some(org) = self.parent_org {
            Some(org.into())
        } else {
            None
        };
        let members = self
            .members
            .iter()
            .map(|acc_share| -> Result<(R::AccountId, R::Shares), C::Error> {
                let mem: Ss58<R> = acc_share.0.parse()?;
                let amt_issued: R::Shares = (acc_share.1).into();
                Ok((mem.0, amt_issued))
            })
            .collect::<Result<Vec<(R::AccountId, R::Shares)>, C::Error>>()?;
        let event = client
            .register_weighted_org(
                sudo,
                parent_org,
                (*self.constitution).to_string(),
                &members,
            )
            .await
            .map_err(Error::Client)?;
        println!(
            "Account {} created a weighted organization with OrgId: {}, constitution: {:?} and {} total shares minted for new members",
            event.caller, event.new_id, event.constitution, event.total
        );
        Ok(())
    }
}
