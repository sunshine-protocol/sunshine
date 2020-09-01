use frame_support::Parameter;
use libipld::{
    cbor::DagCborCodec,
    codec::{
        Decode as DagEncode,
        Encode as DagDecode,
    },
};
use parity_scale_codec::{
    Codec,
    Decode,
    Encode,
};
use sp_runtime::traits::{
    AtLeast32Bit,
    MaybeSerializeDeserialize,
    Member,
    Zero,
};
use std::fmt::Debug;
use substrate_subxt::{
    module,
    sp_runtime,
    system::{
        System,
        SystemEventsDecoder,
    },
    Call,
    Event,
    Store,
};
use sunshine_bounty_utils::{
    organization::{
        Organization,
        Relation,
    },
    share::{
        ProfileState,
        ShareProfile,
    },
};

/// The subset of the org trait and its inherited traits that the client must inherit
#[module]
pub trait Org: System {
    /// Cid type
    type Cid: Parameter + Member + Default;

    /// Organization Identifier
    type OrgId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Zero;

    /// Metric for measuring ownership in context of OrgId (group)
    type Shares: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Zero;

    /// Constitution associated type, text block
    type Constitution: 'static
        + Codec
        + Default
        + Clone
        + DagEncode<DagCborCodec>
        + DagDecode<DagCborCodec>
        + Send
        + Sync;
}

pub type OrgState<T> = Organization<
    <T as System>::AccountId,
    <T as Org>::OrgId,
    <T as Org>::Shares,
    <T as Org>::Cid,
>;
pub type Prof<T> = ShareProfile<
    (<T as Org>::OrgId, <T as System>::AccountId),
    <T as Org>::Shares,
    ProfileState,
>;
pub type Relacion<T> = Relation<<T as Org>::OrgId>;
// ~~ Storage ~~

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct OrgsStore<T: Org> {
    #[store(returns = OrgState<T>)]
    pub org: T::OrgId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct OrgTreeStore<T: Org> {
    #[store(returns = Relacion<T>)]
    pub parent: T::OrgId,
    pub child: T::OrgId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct MembersStore<'a, T: Org> {
    #[store(returns = Prof<T>)]
    pub org: T::OrgId,
    pub who: &'a <T as System>::AccountId,
}

// ~~ (Calls, Events) ~~

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct NewFlatOrgCall<'a, T: Org> {
    pub sudo: Option<<T as System>::AccountId>,
    pub parent_org: Option<T::OrgId>,
    pub constitution: T::Cid,
    pub members: &'a [<T as System>::AccountId],
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct NewFlatOrgEvent<T: Org> {
    pub caller: <T as System>::AccountId,
    pub new_id: T::OrgId,
    pub constitution: T::Cid,
    pub total: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct NewWeightedOrgCall<'a, T: Org> {
    pub sudo: Option<<T as System>::AccountId>,
    pub parent_org: Option<T::OrgId>,
    pub constitution: T::Cid,
    pub weighted_members: &'a [(<T as System>::AccountId, T::Shares)],
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct NewWeightedOrgEvent<T: Org> {
    pub caller: <T as System>::AccountId,
    pub new_id: T::OrgId,
    pub constitution: T::Cid,
    pub total: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct IssueSharesCall<'a, T: Org> {
    pub organization: T::OrgId,
    pub who: &'a <T as System>::AccountId,
    pub shares: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesIssuedEvent<T: Org> {
    pub organization: T::OrgId,
    pub who: <T as System>::AccountId,
    pub shares: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct BurnSharesCall<'a, T: Org> {
    pub organization: T::OrgId,
    pub who: &'a <T as System>::AccountId,
    pub shares: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesBurnedEvent<T: Org> {
    pub organization: T::OrgId,
    pub who: <T as System>::AccountId,
    pub shares: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct BatchIssueSharesCall<'a, T: Org> {
    pub organization: T::OrgId,
    pub new_accounts: &'a [(<T as System>::AccountId, T::Shares)],
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesBatchIssuedEvent<T: Org> {
    pub organization: T::OrgId,
    pub total_new_shares_minted: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct BatchBurnSharesCall<'a, T: Org> {
    pub organization: T::OrgId,
    pub old_accounts: &'a [(<T as System>::AccountId, T::Shares)],
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesBatchBurnedEvent<T: Org> {
    pub organization: T::OrgId,
    pub total_new_shares_burned: T::Shares,
}
