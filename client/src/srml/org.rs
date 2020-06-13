use codec::{Codec, Decode, Encode};
use frame_support::Parameter;
use sp_runtime::traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero};
use std::fmt::Debug;
use substrate_subxt::system::{System, SystemEventsDecoder};
use util::{organization::Organization, share::ShareProfile};

/// The subset of the org trait and its inherited traits that the client must inherit
#[module]
pub trait Org: System {
    /// Cid type
    type IpfsReference: Parameter + Member + Default;

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
}

// ~~ Values ~~

#[derive(Clone, Debug, Eq, PartialEq, Encode)]
pub struct SudoKeyStore<T: Org> {
    pub sudo: Option<<T as System>::AccountId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode)]
pub struct OrganizationIdentifierNonceStore<T: Org> {
    pub nonce: T::OrgId,
}

#[derive(Clone, Debug, Eq, PartialEq, Encode)]
pub struct OrganizationCounterStore {
    pub counter: u32,
}

// ~~ Maps ~~

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct OrgStateStore<T: Org> {
    #[store(returns = Organization<<T as System>::AccountId, T::OrgId, T::IpfsReference>)]
    pub org: T::OrgId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct TotalIssuanceStore<T: Org> {
    #[store(returns = T::Shares)]
    pub org: T::OrgId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct ProfileStore<'a, T: Org> {
    #[store(returns = ShareProfile<T::Shares>)]
    pub org: T::OrgId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct OrganizationSupervisor<T: Org> {
    #[store(returns = Option<<T as System>::AccountId>)]
    pub org: T::OrgId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct OrganizationSizeStore<T: Org> {
    #[store(returns = u32)]
    pub org: T::OrgId,
}

// ~~ (Calls, Events) ~~

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct RegisterFlatOrgCall<'a, T: Org> {
    pub sudo: Option<<T as System>::AccountId>,
    pub parent_org: Option<T::OrgId>,
    pub constitution: T::IpfsReference,
    pub members: &'a [<T as System>::AccountId],
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct FlatOrgRegisteredEvent<T: Org> {
    pub caller: <T as System>::AccountId,
    pub new_id: T::OrgId,
    pub constitution: T::IpfsReference,
    pub total: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct RegisterWeightedOrgCall<'a, T: Org> {
    pub sudo: Option<<T as System>::AccountId>,
    pub parent_org: Option<T::OrgId>,
    pub constitution: T::IpfsReference,
    pub members: &'a [(<T as System>::AccountId, T::Shares)],
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct WeightedOrgRegisteredEvent<T: Org> {
    pub caller: <T as System>::AccountId,
    pub new_id: T::OrgId,
    pub constitution: T::IpfsReference,
    pub total: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct IssueSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub who: &'a <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesIssuedEvent<T: Org> {
    pub org: T::OrgId,
    pub who: <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct BurnSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub who: &'a <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesBurnedEvent<T: Org> {
    pub org: T::OrgId,
    pub who: <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct BatchIssueSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub new_accounts: &'a [(<T as System>::AccountId, T::Shares)],
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesBatchIssuedEvent<T: Org> {
    pub org: T::OrgId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct BatchBurnSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub old_accounts: &'a [(<T as System>::AccountId, T::Shares)],
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesBatchBurnedEvent<T: Org> {
    pub org: T::OrgId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct LockSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesLockedEvent<T: Org> {
    pub org: T::OrgId,
    pub who: <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct UnlockSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesUnlockedEvent<T: Org> {
    pub org: T::OrgId,
    pub who: <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct ReserveSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesReservedEvent<T: Org> {
    pub org: T::OrgId,
    pub who: <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct UnreserveSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesUnReservedEvent<T: Org> {
    pub org: T::OrgId,
    pub who: <T as System>::AccountId,
    pub amount: T::Shares,
}
