use clap::Clap;
use core::fmt::{
    Debug,
    Display,
};
use substrate_subxt::{
    sp_core::crypto::Ss58Codec,
    system::System,
};
use sunshine_bounty_client::{
    org::{
        AccountShare,
        Org,
        OrgClient,
    },
    TextBlock,
};
use sunshine_client_utils::{
    crypto::ss58::Ss58,
    Node,
    Result,
};

#[derive(Clone, Debug, Clap)]
pub struct NewFlatOrgCommand {
    pub constitution: String,
    pub sudo: Option<String>,
    pub parent_org: Option<u64>,
    pub members: Vec<String>,
}

impl NewFlatOrgCommand {
    pub async fn exec<N: Node, C: OrgClient<N>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        N::Runtime: Org,
        <N::Runtime as System>::AccountId: Ss58Codec,
        <N::Runtime as Org>::OrgId: From<u64> + Display,
        <N::Runtime as Org>::Constitution: From<TextBlock>,
    {
        let sudo = if let Some(acc) = &self.sudo {
            let new_acc: Ss58<N::Runtime> = acc.parse()?;
            Some(new_acc.0)
        } else {
            None
        };
        let parent_org: Option<<N::Runtime as Org>::OrgId> = if let Some(org) = self.parent_org {
            Some(org.into())
        } else {
            None
        };
        let constitution = TextBlock {
            text: (*self.constitution).to_string(),
        };
        let members = self
            .members
            .iter()
            .map(|acc|{
                let mem: Ss58<N::Runtime> = acc.parse()?;
                Ok(mem.0)
            })
            .collect::<Result<Vec<_>>>()?;
        let event = client
            .new_flat_org(sudo, parent_org, constitution.into(), &members)
            .await?;
        println!(
            "Account {} created a flat organization with OrgId: {}, constitution: {:?} and {} members of equal ownership weight",
            event.caller, event.new_id, event.constitution, event.total
        );
        Ok(())
    }
}

#[derive(Clone, Debug, Clap)]
pub struct NewWeightedOrgCommand {
    pub constitution: String,
    pub sudo: Option<String>,
    pub parent_org: Option<u64>,
    pub members: Vec<AccountShare>,
}

impl NewWeightedOrgCommand {
    pub async fn exec<N: Node, C: OrgClient<N>>(
        &self,
        client: &C,
    ) -> Result<()>
    where
        N::Runtime: Org,
        <N::Runtime as System>::AccountId: Ss58Codec,
        <N::Runtime as Org>::OrgId: From<u64> + Display,
        <N::Runtime as Org>::Shares: From<u64> + Display,
        <N::Runtime as Org>::Constitution: From<TextBlock>,
    {
        let sudo: Option<<N::Runtime as System>::AccountId> = if let Some(acc) = &self.sudo {
            let new_acc: Ss58<N::Runtime> = acc.parse()?;
            Some(new_acc.0)
        } else {
            None
        };
        let parent_org: Option<<N::Runtime as Org>::OrgId> = if let Some(org) = self.parent_org {
            Some(org.into())
        } else {
            None
        };
        let constitution = TextBlock {
            text: (*self.constitution).to_string(),
        };
        let members = self
            .members
            .iter()
            .map(|acc_share| {
                let mem: Ss58<N::Runtime> = acc_share.0.parse()?;
                let amt_issued: <N::Runtime as Org>::Shares = (acc_share.1).into();
                Ok((mem.0, amt_issued))
            })
            .collect::<Result<Vec<_>>>()?;
        let event = client
            .new_weighted_org(sudo, parent_org, constitution.into(), &members)
            .await?;
        println!(
            "Account {} created a weighted organization with OrgId: {}, constitution: {:?} and {} total shares minted for new members",
            event.caller, event.new_id, event.constitution, event.total
        );
        Ok(())
    }
}
