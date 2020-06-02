use codec::{Codec, Decode, Encode};
use frame_support::Parameter;
use sp_runtime::traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero};
use sp_std::fmt::Debug;
use substrate_subxt::system::{System, SystemEventsDecoder};
//use util::share::AtomicShareProfile;
//use util::uuid::UUID2;

/// The subset of the org trait and its inherited traits that the client must inherit
#[module]
pub trait OrganizationInterface: System {
    // from membership module
    type OrgId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Zero;
    
    // from shares-membership module
    type FlatShareId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Zero;

    // from shares-atomic module
    type WeightedShareId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Zero;

    // from shares-atomic module
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

/*
#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct ShareGroupSupervisorStore<T: SharesAtomic> {
    #[store(returns = <T as System>::AccountId)]
    pub org: T::OrgId,
    pub share: T::ShareId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct ShareIdCounterStore<T: SharesAtomic> {
    #[store(returns = u32)]
    pub org: T::OrgId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct ClaimedShareIdentityStore<T: SharesAtomic> {
    #[store(returns = bool)]
    pub org: T::OrgId,
    pub share: T::ShareId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct MembershipReferenceCounterStore<T: SharesAtomic> {
    #[store(returns = u32)]
    pub org: T::OrgId,
    pub account: <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct TotalIssuanceStore<T: SharesAtomic> {
    #[store(returns = T::Shares)]
    pub org: T::OrgId,
    pub share: T::ShareId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct ProfileStore<'a, T: SharesAtomic> {
    #[store(returns = AtomicShareProfile<T::Shares>)]
    pub prefix: UUID2,
    pub account_id: &'a <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Store, Encode)]
pub struct ShareGroupSizeStore<T: SharesAtomic> {
    #[store(returns = u32)]
    pub org: T::OrgId,
    pub share: T::ShareId,
}
*/

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct IssueSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub who: &'a <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct BurnSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub who: &'a <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct BatchIssueSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub new_accounts: &'a [(<T as System>::AccountId, T::Shares)],
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct BatchBurnSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub new_accounts: &'a [(<T as System>::AccountId, T::Shares)],
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct LockSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct UnlockSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub who: &'a <T as System>::AccountId,
}

/// Request the share reservation.
#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct ReserveSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Call, Encode)]
pub struct UnreserveSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesReservedEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub account: <T as System>::AccountId,
    pub reserved: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesUnReservedEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub account: <T as System>::AccountId,
    pub reserved: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesLockedEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub account: <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesUnlockedEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub account: <T as System>::AccountId,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesIssuedEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub account: <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesBurnedEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub account: <T as System>::AccountId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesBatchIssuedEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct SharesBatchBurnedEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub amount: T::Shares,
}

#[derive(Clone, Debug, Eq, PartialEq, Event, Decode)]
pub struct TotalSharesIssuedEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
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
