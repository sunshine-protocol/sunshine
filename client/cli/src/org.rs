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
use bounty_client::{
    Account,
    AccountShare,
};
use clap::Clap;
use core::fmt::{
    Debug,
    Display,
};
use libipld::{
    cid::{
        Cid,
        Codec,
    },
    multihash::Blake2b256,
};
use substrate_subxt::{
    sp_core::crypto::Ss58Codec,
    system::System,
};
use utils_identity::cid::CidBytes;

#[derive(Clone, Debug, Clap)]
pub struct OrgRegisterFlatCommand {
    pub sudo: Option<String>,
    pub parent_org: Option<u64>,
    pub constitution: String,
    pub members: Vec<String>,
}

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for OrgRegisterFlatCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Org>::Shares: From<u64> + Display,
    <T as Org>::IpfsReference: From<CidBytes> + Debug,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let sudo: Option<T::AccountId> = if let Some(acc) = &self.sudo {
            let new_acc: Account<T> = acc.parse()?;
            Some(new_acc.id)
        } else {
            None
        };
        let parent_org: Option<T::OrgId> = if let Some(org) = self.parent_org {
            Some(org.into())
        } else {
            None
        };
        let constitution: CidBytes = {
            let content = self.constitution.as_bytes();
            let hash = Blake2b256::digest(&content[..]);
            let cid = Cid::new_v1(Codec::Raw, hash);
            CidBytes::from(&cid)
        };
        let members = self
            .members
            .iter()
            .map(|acc| -> Result<T::AccountId> {
                let mem: Account<T> = acc.parse()?;
                Ok(mem.id)
            })
            .collect::<Result<Vec<T::AccountId>>>()?;
        let event = client
            .register_flat_org(sudo, parent_org, constitution.into(), &members)
            .await?;
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

#[async_trait]
impl<T: Runtime + Org + Vote + Donate + Bank + Bounty, P: Pair> Command<T, P>
    for OrgRegisterWeightedCommand
where
    <T as System>::AccountId: Ss58Codec,
    <T as Org>::OrgId: From<u64> + Display,
    <T as Org>::Shares: From<u64> + Display,
    <T as Org>::IpfsReference: From<CidBytes> + Debug,
{
    async fn exec(&self, client: &dyn AbstractClient<T, P>) -> Result<()> {
        let sudo: Option<T::AccountId> = if let Some(acc) = &self.sudo {
            let new_acc: Account<T> = acc.parse()?;
            Some(new_acc.id)
        } else {
            None
        };
        let parent_org: Option<T::OrgId> = if let Some(org) = self.parent_org {
            Some(org.into())
        } else {
            None
        };
        let constitution: CidBytes = {
            let content = self.constitution.as_bytes();
            let hash = Blake2b256::digest(&content[..]);
            let cid = Cid::new_v1(Codec::Raw, hash);
            CidBytes::from(&cid)
        };
        let members = self
            .members
            .iter()
            .map(|acc_share| -> Result<(T::AccountId, T::Shares)> {
                let mem: Account<T> = acc_share.0.parse()?;
                let amt_issued: T::Shares = (acc_share.1).into();
                Ok((mem.id, amt_issued))
            })
            .collect::<Result<Vec<(T::AccountId, T::Shares)>>>()?;
        let event = client
            .register_weighted_org(
                sudo,
                parent_org,
                constitution.into(),
                &members,
            )
            .await?;
        println!(
            "Account {} created a weighted organization with OrgId: {}, constitution: {:?} and {} total shares minted for new members",
            event.caller, event.new_id, event.constitution, event.total
        );
        Ok(())
    }
}
