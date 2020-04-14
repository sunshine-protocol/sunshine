use codec::{Decode, Encode};
use frame_support::Parameter;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(PartialEq, Eq, Default, Copy, Clone, Encode, Decode, RuntimeDebug)]
pub struct OrgItemPrefixKey<OrgId, FirstIdentifier> {
    org: OrgId,
    first_id: FirstIdentifier,
}

impl<OrgId: Parameter + Copy, FirstIdentifier: From<u32> + Copy>
    OrgItemPrefixKey<OrgId, FirstIdentifier>
{
    pub fn new(org: OrgId, first_id: FirstIdentifier) -> OrgItemPrefixKey<OrgId, FirstIdentifier> {
        OrgItemPrefixKey { org, first_id }
    }
    pub fn org(&self) -> OrgId {
        self.org
    }
    pub fn first_id(&self) -> FirstIdentifier {
        self.first_id
    }
}

#[derive(PartialEq, Eq, Default, Copy, Clone, Encode, Decode, RuntimeDebug)]
/// Used in vote and bounty to track relevant state for votes and bounties in each
pub struct OrgTwoItemPrefixKey<OrgId, FirstIdentifier, SecondIdentifier> {
    org: OrgId,
    first_id: FirstIdentifier,
    second_id: SecondIdentifier,
}

impl<
        OrgId: Parameter + Copy,
        FirstIdentifier: From<u32> + Copy,
        SecondIdentifier: From<u32> + Copy,
    > OrgTwoItemPrefixKey<OrgId, FirstIdentifier, SecondIdentifier>
{
    pub fn new(
        org: OrgId,
        first_id: FirstIdentifier,
        second_id: SecondIdentifier,
    ) -> OrgTwoItemPrefixKey<OrgId, FirstIdentifier, SecondIdentifier> {
        OrgTwoItemPrefixKey {
            org,
            first_id,
            second_id,
        }
    }
    pub fn org_item_prefix(&self) -> OrgItemPrefixKey<OrgId, FirstIdentifier> {
        OrgItemPrefixKey::new(self.org, self.first_id)
    }
    pub fn org(&self) -> OrgId {
        self.org
    }
    pub fn first_id(&self) -> FirstIdentifier {
        self.first_id
    }
    pub fn second_id(&self) -> SecondIdentifier {
        self.second_id
    }
}
