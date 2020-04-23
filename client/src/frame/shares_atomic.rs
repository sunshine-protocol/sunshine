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

#[storage]
pub trait SharesAtomicStore<T: SharesAtomic> {
    fn share_id_counter(org: &T::OrgId) -> T::ShareId;

    fn total_issuance(org: &T::OrgId, share: &T::ShareId) -> T::Share;

    fn share_holders(org: &T::OrgId, share: &T::ShareId) -> Vec<<T as System>::AccountId>;

    fn profile(
        prefix: &(&T::OrgId, &T::ShareId),
        account_id: &<T as System>::AccountId,
    ) -> AtomicShareProfile<T::Share>;
}

/// Register shares.
#[derive(Call, Debug, Encode)]
pub struct RegisterSharesCall<T: SharesAtomic> {
    organization: T::OrgId,
    share_id: T::ShareId,
    genesis: Vec<(T::AccountId, T::Share)>,
}

#[derive(Call, Debug, Encode)]
pub struct LockSharesCall<T: SharesAtomic> {
    organization: T::OrgId,
    share_id: T::ShareId,
    who: <T as System>::AccountId,
}

#[derive(Call, Debug, Encode)]
pub struct UnlockSharesCall<T: SharesAtomic> {
    organization: T::OrgId,
    share_id: T::ShareId,
    who: <T as System>::AccountId,
}

/// Request the share reservation.
#[derive(Call, Debug, Encode)]
pub struct ReserveSharesCall<T: SharesAtomic> {
    organization: T::OrgId,
    share_id: T::ShareId,
    who: <T as System>::AccountId,
}

#[derive(Call, Debug, Encode)]
pub struct UnreserveSharesCall<T: SharesAtomic> {
    organization: T::OrgId,
    share_id: T::ShareId,
    who: <T as System>::AccountId,
}

#[derive(Call, Debug, Encode)]
pub struct IssueSharesCall<T: SharesAtomic> {
    organization: T::OrgId,
    share_id: T::ShareId,
    who: <T as System>::AccountId,
    shares: T::Share,
}

#[derive(Call, Debug, Encode)]
pub struct BurnSharesCall<T: SharesAtomic> {
    organization: T::OrgId,
    share_id: T::ShareId,
    who: <T as System>::AccountId,
    shares: T::Share,
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
    use sp_core::crypto::Pair;
    use sp_keyring::AccountKeyring;

    #[async_std::test]
    #[ignore]
    async fn test_reserve_shares() {
        env_logger::init();
        let signer = AccountKeyring::Eve.pair();
        let client = crate::build_client().await.unwrap();
        let xt = client.xt(signer.clone(), None).await.unwrap();

        let org = 1;
        let share = 1;
        let account = signer.public().into();
        let reserved = client
            .profile(&(&org, &share), &account)
            .await
            .unwrap()
            .get_times_reserved();

        let extrinsic_success = xt
            .watch()
            .with_shares_atomic()
            .reserve_shares(org, share, account.clone())
            .await
            .unwrap();
        let event = extrinsic_success.shares_reserved().unwrap().unwrap();
        assert_eq!(
            event,
            SharesReservedEvent {
                org,
                share,
                account,
                reserved: reserved + 1
            }
        );
    }
}
