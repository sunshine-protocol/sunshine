use codec::{Codec, Decode, Encode};
use frame_support::Parameter;
use sp_runtime::traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero};
use std::fmt::Debug;
use substrate_subxt::system::System;
use substrate_subxt_proc_macro::*;
use util::share::AtomicShareProfile;

/// The subset of the `shares_atomic::Trait` that a client must implement.
#[module]
pub trait SharesAtomic: System {
    type OrgId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Zero;

    type ShareId: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug;

    type Share: Parameter
        + Member
        + AtLeast32Bit
        + Codec
        + Default
        + Copy
        + MaybeSerializeDeserialize
        + Debug
        + Zero;
}

#[derive(Encode, Store)]
pub struct ShareIdCounterStore<T: SharesAtomic> {
    #[store(returns = T::ShareId)]
    pub org: T::OrgId,
}

#[derive(Encode, Store)]
pub struct TotalIssuanceStore<T: SharesAtomic> {
    #[store(returns = T::Share)]
    pub org: T::OrgId,
    pub share: T::ShareId,
}

#[derive(Encode, Store)]
pub struct ShareHoldersStore<T: SharesAtomic> {
    #[store(returns = Vec<<T as System>::AccountId>)]
    pub org: T::OrgId,
    pub share: T::ShareId,
}

#[derive(Encode, Store)]
pub struct ProfileStore<'a, T: SharesAtomic> {
    #[store(returns = AtomicShareProfile<T::Share>)]
    pub prefix: (T::OrgId, T::ShareId),
    pub account_id: &'a <T as System>::AccountId,
}

/// Register shares.
#[derive(Call, Debug, Encode)]
pub struct RegisterSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub genesis: &'a [(T::AccountId, T::Share)],
}

#[derive(Call, Debug, Encode)]
pub struct LockSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Call, Debug, Encode)]
pub struct UnlockSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub who: &'a <T as System>::AccountId,
}

/// Request the share reservation.
#[derive(Call, Debug, Encode)]
pub struct ReserveSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Call, Debug, Encode)]
pub struct UnreserveSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub who: &'a <T as System>::AccountId,
}

#[derive(Call, Debug, Encode)]
pub struct IssueSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub who: &'a <T as System>::AccountId,
    pub amount: T::Share,
}

#[derive(Call, Debug, Encode)]
pub struct BurnSharesCall<'a, T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub who: &'a <T as System>::AccountId,
    pub amount: T::Share,
}

#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct SharesReservedEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub account: <T as System>::AccountId,
    pub reserved: u32,
}

#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct SharesUnReservedEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub account: <T as System>::AccountId,
    pub reserved: u32,
}

#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct SharesLockedEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub account: <T as System>::AccountId,
}

#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct SharesUnlockedEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub account: <T as System>::AccountId,
}

#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct NewShareTypeEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
}

#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct IssuanceEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub account: <T as System>::AccountId,
    pub amount: T::Share,
}

#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct BurnEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub account: <T as System>::AccountId,
    pub amount: T::Share,
}

#[derive(Debug, Decode, Eq, Event, PartialEq)]
pub struct TotalSharesIssuedEvent<T: SharesAtomic> {
    pub org: T::OrgId,
    pub share: T::ShareId,
    pub amount: T::Share,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Runtime, RuntimeExtra};

    subxt_test!({
        name: register_shares,
        runtime: Runtime,
        extra: RuntimeExtra,
        step: {
            state: {
                counter: ShareIdCounterStore {
                    org: 1,
                },
            },
            call: RegisterSharesCall {
                org: 1,
                share: 1,
                genesis: &[(alice.clone(), 10), (bob.clone(), 10)],
            },
            event: NewShareTypeEvent {
                org: 1,
                share: pre.counter,
            },
            assert: {
                assert_eq!(pre.counter + 1, post.counter);
            },
        },
    });

    subxt_test!({
        name: reserve_shares,
        runtime: Runtime,
        extra: RuntimeExtra,
        state: {
            profile: ProfileStore {
                prefix: (1, 1),
                account_id: &alice,
            },
        },
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
                reserved: pre.profile.get_times_reserved() + 1,
            },
            assert: {
                assert_eq!(pre.profile.get_times_reserved() + 1, post.profile.get_times_reserved());
            },
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
                reserved: pre.profile.get_times_reserved() - 1,
            },
            assert: {
                assert_eq!(pre.profile.get_times_reserved() - 1, post.profile.get_times_reserved());
            },
        }
    });

    subxt_test!({
        name: lock_shares,
        runtime: Runtime,
        extra: RuntimeExtra,
        state: {
            profile: ProfileStore {
                prefix: (1, 1),
                account_id: &alice,
            },
        },
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
            assert: {
                assert_eq!(pre.profile.is_unlocked(), true);
                assert_eq!(post.profile.is_unlocked(), false);
            },
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
            assert: {
                assert_eq!(pre.profile.is_unlocked(), false);
                assert_eq!(post.profile.is_unlocked(), true);
            },
        }
    });

    subxt_test!({
        name: issue_shares,
        runtime: Runtime,
        extra: RuntimeExtra,
        state: {
            issuance: TotalIssuanceStore {
                org: 1,
                share: 1,
            },
        },
        step: {
            call: IssueSharesCall {
                org: 1,
                share: 1,
                who: &alice,
                amount: 10,
            },
            event: IssuanceEvent {
                org: 1,
                share: 1,
                account: alice.clone(),
                amount: 10,
            },
            assert: {
                assert_eq!(pre.issuance + 10, post.issuance);
            },
        },
        step: {
            call: BurnSharesCall {
                org: 1,
                share: 1,
                who: &alice,
                amount: 10,
            },
            event: BurnEvent {
                org: 1,
                share: 1,
                account: alice.clone(),
                amount: 10,
            },
            assert: {
                assert_eq!(pre.issuance - 10, post.issuance);
            },
        }
    });
}
