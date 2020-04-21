use codec::{Codec, Error as CodecError};
use frame_support::Parameter;
use futures::future;
use sp_runtime::traits::{AtLeast32Bit, MaybeSerializeDeserialize, Member, Zero};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use substrate_subxt::{system::System, Call, Client, EventsSubscriber, ExtrinsicSuccess};
use util::share::AtomicShareProfile;

/// The subset of the `shares_atomic::Trait` that a client must implement.
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

pub trait SharesAtomicEventsDecoder {
    fn with_shares_atomic(self) -> Self;
}

impl<T: SharesAtomic, P, S, E> SharesAtomicEventsDecoder for EventsSubscriber<T, P, S, E> {
    fn with_shares_atomic(self) -> Self {
        self.events_decoder(|decoder| {
            decoder.register_type_size::<T::OrgId>("OrgId")?;
            decoder.register_type_size::<T::ShareId>("ShareId")?;
            decoder.register_type_size::<T::Share>("Share")
        })
    }
}

const MODULE: &str = "SharesAtomic";

/// Register shares.
pub fn register_shares<T: SharesAtomic>(
    org: T::OrgId,
    share: T::ShareId,
    genesis: Vec<(T::AccountId, T::Share)>,
) -> Call<(T::OrgId, T::ShareId, Vec<(T::AccountId, T::Share)>)> {
    Call::new(MODULE, "register_shares", (org, share, genesis))
}

/// Request the share reservation.
pub fn reserve_shares<T: SharesAtomic>(
    org: T::OrgId,
    share: T::ShareId,
    account: <T as System>::AccountId,
) -> Call<(T::OrgId, T::ShareId, <T as System>::AccountId)> {
    Call::new(MODULE, "reserve_shares", (org, share, account))
}

type EventResult<T> = Option<Result<T, CodecError>>;

pub trait SharesAtomicEvents<T: SharesAtomic> {
    fn shares_reserved(&self)
        -> EventResult<(T::OrgId, T::ShareId, <T as System>::AccountId, u32)>;
    fn shares_unreserved(
        &self,
    ) -> EventResult<(T::OrgId, T::ShareId, <T as System>::AccountId, u32)>;
    fn shares_locked(&self) -> EventResult<(T::OrgId, T::ShareId, <T as System>::AccountId)>;
    fn shares_unlocked(&self) -> EventResult<(T::OrgId, T::ShareId, <T as System>::AccountId)>;
    fn new_share_type(&self) -> EventResult<(T::OrgId, T::ShareId)>;
    fn issuance(&self) -> EventResult<(T::OrgId, T::ShareId, <T as System>::AccountId, T::Share)>;
    fn burn(&self) -> EventResult<(T::OrgId, T::ShareId, <T as System>::AccountId, T::Share)>;
    fn total_shares_issued(&self) -> EventResult<(T::OrgId, T::ShareId, T::Share)>;
}

impl<T: SharesAtomic> SharesAtomicEvents<T> for ExtrinsicSuccess<T> {
    fn shares_reserved(
        &self,
    ) -> EventResult<(T::OrgId, T::ShareId, <T as System>::AccountId, u32)> {
        self.find_event::<(T::OrgId, T::ShareId, <T as System>::AccountId, u32)>(
            MODULE,
            "SharesReserved",
        )
    }

    fn shares_unreserved(
        &self,
    ) -> EventResult<(T::OrgId, T::ShareId, <T as System>::AccountId, u32)> {
        self.find_event::<(T::OrgId, T::ShareId, <T as System>::AccountId, u32)>(
            MODULE,
            "SharesUnReserved",
        )
    }

    fn shares_locked(&self) -> EventResult<(T::OrgId, T::ShareId, <T as System>::AccountId)> {
        self.find_event::<(T::OrgId, T::ShareId, <T as System>::AccountId)>(MODULE, "SharesLocked")
    }

    fn shares_unlocked(&self) -> EventResult<(T::OrgId, T::ShareId, <T as System>::AccountId)> {
        self.find_event::<(T::OrgId, T::ShareId, <T as System>::AccountId)>(
            MODULE,
            "SharesUnlocked",
        )
    }

    fn new_share_type(&self) -> EventResult<(T::OrgId, T::ShareId)> {
        self.find_event::<(T::OrgId, T::ShareId)>(MODULE, "NewShareType")
    }

    fn issuance(&self) -> EventResult<(T::OrgId, T::ShareId, <T as System>::AccountId, T::Share)> {
        self.find_event::<(T::OrgId, T::ShareId, <T as System>::AccountId, T::Share)>(
            MODULE, "Issuance",
        )
    }

    fn burn(&self) -> EventResult<(T::OrgId, T::ShareId, <T as System>::AccountId, T::Share)> {
        self.find_event::<(T::OrgId, T::ShareId, <T as System>::AccountId, T::Share)>(
            MODULE, "Burn",
        )
    }

    fn total_shares_issued(&self) -> EventResult<(T::OrgId, T::ShareId, T::Share)> {
        self.find_event::<(T::OrgId, T::ShareId, T::Share)>(MODULE, "TotalSharesIssued")
    }
}

type StoreResult<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, substrate_subxt::Error>> + Send + 'a>>;

pub trait SharesAtomicStore<T: SharesAtomic> {
    fn total_issuance<'a>(
        &'a self,
        org: &T::OrgId,
        share: &T::ShareId,
    ) -> StoreResult<'a, T::Share>;
    fn profile<'a>(
        &'a self,
        org: &T::OrgId,
        share: &T::ShareId,
        account_id: &<T as System>::AccountId,
    ) -> StoreResult<'a, AtomicShareProfile<T::Share>>;
}

impl<T, S, E> SharesAtomicStore<T> for Client<T, S, E>
where
    T: SharesAtomic + Send + Sync,
    S: 'static,
    E: Send + Sync + 'static,
{
    fn total_issuance<'a>(
        &'a self,
        org: &T::OrgId,
        share: &T::ShareId,
    ) -> StoreResult<'a, T::Share> {
        let map_fn = || {
            Ok(self
                .metadata()
                .module(MODULE)?
                .storage("TotalIssuance")?
                .map()?)
        };
        let map = match map_fn() {
            Ok(map) => map,
            Err(err) => return Box::pin(future::err(err)),
        };
        let future = self.fetch(map.key(&(org, share)), None);
        Box::pin(async move {
            let v = if let Some(v) = future.await? {
                Some(v)
            } else {
                map.default().cloned()
            };
            Ok(v.unwrap_or_default())
        })
    }

    fn profile<'a>(
        &'a self,
        org: &T::OrgId,
        share: &T::ShareId,
        account: &<T as System>::AccountId,
    ) -> StoreResult<'a, AtomicShareProfile<T::Share>> {
        let map_fn = || {
            Ok(self
                .metadata()
                .module(MODULE)?
                .storage("Profile")?
                .double_map()?)
        };
        let map = match map_fn() {
            Ok(map) => map,
            Err(err) => return Box::pin(future::err(err)),
        };
        let future = self.fetch(map.key(&(org, share), account), None);
        Box::pin(async move {
            let v = if let Some(v) = future.await? {
                Some(v)
            } else {
                map.default().cloned()
            };
            Ok(v.unwrap_or_default())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Runtime;
    use sp_core::crypto::Pair;
    use sp_keyring::AccountKeyring;

    #[async_std::test]
    async fn test_reserve_shares() {
        env_logger::init();
        let signer = AccountKeyring::Eve.pair();
        let client = crate::build_client().await.unwrap();
        let xt = client.xt(signer.clone(), None).await.unwrap();

        let org = 1;
        let share = 1;
        let account = signer.public().into();
        let reserved = client
            .profile(&org, &share, &account)
            .await
            .unwrap()
            .get_times_reserved();

        let extrinsic_success = xt
            .watch()
            .with_shares_atomic()
            .submit(reserve_shares::<Runtime>(org, share, account.clone()))
            .await
            .unwrap();
        let res = extrinsic_success.shares_reserved().unwrap().unwrap();
        assert_eq!((org, share, account, reserved + 1), res);
    }
}
