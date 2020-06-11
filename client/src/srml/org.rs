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
    pub account: &'a <T as System>::AccountId,
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

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct IssueSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub who: &'a <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct BurnSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub who: &'a <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct BatchIssueSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub new_accounts: &'a [(<T as System>::AccountId, T::Shares)],
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct BatchBurnSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub new_accounts: &'a [(<T as System>::AccountId, T::Shares)],
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct LockSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct UnlockSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct ReserveSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct UnreserveSharesCall<'a, T: Org> {
    pub org: T::OrgId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesReservedEvent<T: Org> {
    pub org: T::OrgId,
    pub who: <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesUnReservedEvent<T: Org> {
    pub org: T::OrgId,
    pub who: <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesLockedEvent<T: Org> {
    pub org: T::OrgId,
    pub who: <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesUnlockedEvent<T: Org> {
    pub org: T::OrgId,
    pub who: <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesIssuedEvent<T: Org> {
    pub org: T::OrgId,
    pub account: <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesBurnedEvent<T: Org> {
    pub org: T::OrgId,
    pub account: <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesBatchIssuedEvent<T: Org> {
    pub org: T::OrgId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesBatchBurnedEvent<T: Org> {
    pub org: T::OrgId,
    pub amount: T::Shares,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{Runtime, RuntimeExtra};

    subxt_test!({
        name: issue_shares,
        runtime: Runtime,
        extra: RuntimeExtra,
        /*state: {
            issuance: TotalIssuanceStore {
                org: 1,
                share: 1,
            },
        },*/
        step: {
            call: IssueSharesCall {
                org: 1,
                share: 1,
                who: &alice,
                amount: 10,
            },
            event: SharesIssuedEvent {
                org: 1,
                share: 1,
                account: alice.clone(),
                amount: 10,
            },
            /*assert: {
                assert_eq!(pre.issuance + 20, post.issuance);
            },*/
        },
        step: {
            call: BurnSharesCall {
                org: 1,
                share: 1,
                who: &alice,
                amount: 10,
            },
            event: SharesBurnedEvent {
                org: 1,
                share: 1,
                account: alice.clone(),
                amount: 10,
            },
            /*assert: {
                assert_eq!(pre.issuance - 20, post.issuance);
            },*/
        }
    });

    subxt_test!({
        name: batch_issue_shares,
        runtime: Runtime,
        extra: RuntimeExtra,
        /*state: {
            issuance: TotalIssuanceStore {
                org: 1,
                share: 1,
            },
        },*/
        step: {
            call: BatchIssueSharesCall {
                org: 1,
                share: 1,
                new_accounts: &[(alice.clone(), 10), (bob.clone(), 10)],
            },
            event: SharesBatchIssuedEvent {
                org: 1,
                share: 1,
                amount: 20,
            },
            /*assert: {
                assert_eq!(pre.issuance + 20, post.issuance);
            },*/
        },
        step: {
            call: BatchBurnSharesCall {
                org: 1,
                share: 1,
                new_accounts: &[(alice.clone(), 10), (bob.clone(), 10)],
            },
            event: SharesBatchBurnedEvent {
                org: 1,
                share: 1,
                amount: 20,
            },
            /*assert: {
                assert_eq!(pre.issuance - 20, post.issuance);
            },*/
        }
    });

    subxt_test!({
        name: reserve_shares,
        runtime: Runtime,
        extra: RuntimeExtra,
        /*state: {
            profile: ProfileStore {
                prefix: UUID2::new(1, 1),
                account_id: &alice,
            },
        },*/
        step: {
            call: ReserveSharesCall {
                org: 1,
                share: 1,
                who: &alice,
            },
            event: SharesReservedEvent {
                org: 1,
                share: 1,
                account: alice.clone(),
                reserved: event.reserved, //pre.profile.times_reserved() + 1,
            },
            /*assert: {
                assert_eq!(pre.profile.times_reserved() + 1, post.profile.times_reserved());
            },*/
        },
        step: {
            call: UnreserveSharesCall {
                org: 1,
                share: 1,
                who: &alice,
            },
            event: SharesUnReservedEvent {
                org: 1,
                share: 1,
                account: alice.clone(),
                reserved: event.reserved, //pre.profile.times_reserved() - 1,
            },
            /*assert: {
                assert_eq!(pre.profile.times_reserved() - 1, post.profile.times_reserved());
            },*/
        }
    });

    subxt_test!({
        name: lock_shares,
        runtime: Runtime,
        extra: RuntimeExtra,
        /*state: {
            profile: ProfileStore {
                prefix: UUID2::new(1, 1),
                account_id: &alice,
            },
        },*/
        step: {
            call: LockSharesCall {
                org: 1,
                share: 1,
                who: &alice,
            },
            event: SharesLockedEvent {
                org: 1,
                share: 1,
                account: alice.clone(),
            },
            /*assert: {
                assert_eq!(pre.profile.is_unlocked(), true);
                assert_eq!(post.profile.is_unlocked(), false);
            },*/
        },
        step: {
            call: UnlockSharesCall {
                org: 1,
                share: 1,
                who: &alice,
            },
            event: SharesUnlockedEvent {
                org: 1,
                share: 1,
                account: alice.clone(),
            },
            /*assert: {
                assert_eq!(pre.profile.is_unlocked(), false);
                assert_eq!(post.profile.is_unlocked(), true);
            },*/
        }
    });
}
