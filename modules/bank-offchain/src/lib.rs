#![allow(clippy::string_lit_as_bytes)]
#![allow(clippy::redundant_closure_call)]
#![allow(clippy::type_complexity)]
#![cfg_attr(not(feature = "std"), no_std)]
//! back to [`util`](../util/index.html) for all object and trait definitions

#[cfg(test)]
mod tests;

use util::{
    bank::{OffChainTreasuryID, Payment, PaymentConfirmation},
    court::Evidence,
    organization::{FormedOrganization, ShareID},
    traits::{
        GenerateUniqueID, GenerateUniqueKeyID, GetInnerOuterShareGroups, IDIsAvailable,
        OffChainBank, OrgChecks, OrganizationDNS, RegisterOffChainBankAccount, RegisterShareGroup,
        ShareGroupChecks, SupervisorPermissions, SupportedOrganizationShapes,
        WeightedShareIssuanceWrapper, WeightedShareWrapper,
    },
};

use frame_support::{decl_error, decl_event, decl_module, decl_storage, ensure, traits::Currency};
use frame_system::{self as system, ensure_signed};
use sp_runtime::{DispatchError, DispatchResult, Permill};
use sp_std::prelude::*;

/// Common ipfs type alias for our modules
pub type IpfsReference = Vec<u8>;
/// The organization identfier
pub type OrgId = u32;
/// The weighted shares
pub type SharesOf<T> = <<T as Trait>::Organization as WeightedShareWrapper<
    u32,
    u32,
    <T as frame_system::Trait>::AccountId,
>>::Shares;
/// The balances type for this module, some currency metric
type BalanceOf<T> =
    <<T as Trait>::Currency as Currency<<T as frame_system::Trait>::AccountId>>::Balance;

pub trait Trait: system::Trait {
    type Event: From<Event<Self>> + Into<<Self as frame_system::Trait>::Event>;

    type Currency: Currency<Self::AccountId>;

    type Organization: OrgChecks<u32, Self::AccountId>
        + ShareGroupChecks<u32, ShareID, Self::AccountId>
        + GetInnerOuterShareGroups<u32, ShareID, Self::AccountId>
        + SupervisorPermissions<u32, ShareID, Self::AccountId>
        + WeightedShareWrapper<u32, u32, Self::AccountId>
        + WeightedShareIssuanceWrapper<u32, u32, Self::AccountId, Permill>
        + RegisterShareGroup<u32, ShareID, Self::AccountId, SharesOf<Self>>
        + OrganizationDNS<u32, Self::AccountId, IpfsReference>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as frame_system::Trait>::AccountId,
        Balance = BalanceOf<T>,
    {
        /// New off chain treasury identifier events
        NewOffChainTreasuryRegisteredForOrg(u32, OffChainTreasuryID),
        NewOffChainTreasuryRegisteredForFlatShareGroup(u32, u32, OffChainTreasuryID),
        NewOffChainTreasuryRegisteredForWeightedShareGroup(u32, u32, OffChainTreasuryID),
        /// Off chain treasury usage events
        /// off_chain_treasury_id, sender, recipient, amount, salt associated with the payment
        SenderClaimsPaymentSent(OffChainTreasuryID, AccountId, AccountId, Balance, u32),
        RecipientConfirmsPaymentReceived(OffChainTreasuryID, AccountId, AccountId, Balance, u32),
    }
);

decl_error! {
    pub enum Error for Module<T: Trait> {
        MustHaveCertainAuthorityToRegisterOffChainBankAccountForOrg,
        CannotUseOffChainBankThatDNE,
        FlatShareGroupNotFound,
        WeightedShareGroupNotFound,
        MustBeAMemberToUseOffChainBankAccountToClaimPaymentSent,
        SenderMustClaimPaymentSentForRecipientToConfirm,
        CannotRegisterOffChainBankForOrgThatDNE,
    }
}

decl_storage! {
    trait Store for Module<T: Trait> as Bank {
        /// Nonce for not inefficient UUID generation in IdIsAvailable impl
        OffChainTreasuryIDNonce get(fn off_chain_treasury_id_nonce): u32;

        /// Source of truth for OffChainTreasuryId uniqueness checks
        /// - anyone in the FormedOrganization can use this to keep track of payments made off chain
        OffChainTreasuryIDs get(fn off_chain_treasury_ids): map
            hasher(opaque_blake2_256) OffChainTreasuryID => Option<FormedOrganization>;

        /// Evidence for off-chain transfers
        OffChainTransferEvidence get(fn off_chain_transfer_evidence): double_map
            hasher(opaque_blake2_256) OffChainTreasuryID,
            hasher(opaque_blake2_256) Evidence<T::AccountId, IpfsReference> => bool;

        /// Payment state for off-chain transfers
        OffChainTransfers get(fn off_chain_transfers): double_map
            hasher(opaque_blake2_256) OffChainTreasuryID,
            hasher(opaque_blake2_256) Payment<T::AccountId, BalanceOf<T>> => Option<PaymentConfirmation>;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        type Error = Error<T>;
        fn deposit_event() = default;

        #[weight = 0]
        fn register_offchain_bank_account_for_organization(
            origin,
            organization: OrgId,
        ) -> DispatchResult {
            let caller = ensure_signed(origin)?;
            let authentication: bool = Self::is_sudo_account(&caller)
                || Self::is_organization_supervisor(organization, &caller);
            ensure!(authentication, Error::<T>::MustHaveCertainAuthorityToRegisterOffChainBankAccountForOrg);
            let organization_exists = <<T as Trait>::Organization as OrgChecks<
                    u32,
                    T::AccountId
                >>::check_org_existence(organization);
            ensure!(organization_exists, Error::<T>::CannotRegisterOffChainBankForOrgThatDNE);

            let formed_org: FormedOrganization = organization.into();
            let off_chain_treasury_id = Self::register_off_chain_bank_account(formed_org)?;
            Self::deposit_event(RawEvent::NewOffChainTreasuryRegisteredForOrg(organization, off_chain_treasury_id));
            Ok(())
        }
        #[weight = 0]
        fn register_offchain_bank_account_for_inner_flat_share_group(
            origin,
            organization: u32,
            flat_share_group_id: u32,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            // TODO: could check that this is sudo or organization's supervisor
            let share_id = ShareID::Flat(flat_share_group_id);
            let formed_org: FormedOrganization = (organization, share_id).into();
            // TODO: could check flat share group's existence here
            let off_chain_treasury_id = Self::register_off_chain_bank_account(formed_org)?;
            Self::deposit_event(RawEvent::NewOffChainTreasuryRegisteredForFlatShareGroup(organization, flat_share_group_id, off_chain_treasury_id));
            Ok(())
        }
        #[weight = 0]
        fn register_offchain_bank_account_for_weighted_share_group(
            origin,
            organization: u32,
            weighted_share_group_id: u32,
        ) -> DispatchResult {
            let _ = ensure_signed(origin)?;
            // TODO: could check that this is sudo or organization's supervisor
            let share_id = ShareID::WeightedAtomic(weighted_share_group_id);
            let formed_org: FormedOrganization = (organization, share_id).into();
            // TODO: could check flat share group's existence here
            let off_chain_treasury_id = Self::register_off_chain_bank_account(formed_org)?;
            Self::deposit_event(RawEvent::NewOffChainTreasuryRegisteredForWeightedShareGroup(organization, weighted_share_group_id, off_chain_treasury_id));
            Ok(())
        }
        #[weight = 0]
        fn use_offchain_bank_account_to_claim_payment_sent(
            origin,
            treasury_id: u32,
            recipient: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let sender = ensure_signed(origin)?;
            // get the organization permissions from the off_chain_treasury_id
            let formed_org = OffChainTreasuryIDs::get(treasury_id).ok_or(Error::<T>::CannotUseOffChainBankThatDNE)?;
            // only requirement is that the claimER is in the organization
            let authentication: bool = Self::account_is_member_of_formed_organization(formed_org, &sender);
            ensure!(authentication, Error::<T>::MustBeAMemberToUseOffChainBankAccountToClaimPaymentSent);

            let payment_claimed_sent = Payment::new(0u32, sender.clone(), recipient.clone(), amount);
            let new_payment_salt = Self::sender_claims_payment_sent(treasury_id, payment_claimed_sent).salt();
            Self::deposit_event(RawEvent::SenderClaimsPaymentSent(treasury_id, sender, recipient, amount, new_payment_salt));
            Ok(())
        }
        #[weight = 0]
        fn use_offchain_bank_account_to_confirm_payment_received(
            origin,
            treasury_id: u32,
            salt: u32,
            sender: T::AccountId,
            amount: BalanceOf<T>,
        ) -> DispatchResult {
            let recipient = ensure_signed(origin)?;
            // existence check
            let _ = OffChainTreasuryIDs::get(treasury_id).ok_or(Error::<T>::CannotUseOffChainBankThatDNE)?;
            let payment_confirmed_received = Payment::new(salt, sender.clone(), recipient.clone(), amount);
            Self::recipient_confirms_payment_received(treasury_id, payment_confirmed_received)?;
            Self::deposit_event(RawEvent::RecipientConfirmsPaymentReceived(treasury_id, sender, recipient, amount, salt));
            Ok(())
        }
    }
}

impl<T: Trait> Module<T> {
    fn is_sudo_account(who: &T::AccountId) -> bool {
        <<T as Trait>::Organization as SupervisorPermissions<u32, ShareID, T::AccountId>>::is_sudo_account(
            who,
        )
    }
    fn is_organization_supervisor(organization: u32, who: &T::AccountId) -> bool {
        <<T as Trait>::Organization as SupervisorPermissions<u32, ShareID, T::AccountId>>::is_organization_supervisor(organization, who)
    }
    // fn is_share_supervisor(organization: u32, share_id: ShareID, who: &T::AccountId) -> bool {
    //     <<T as Trait>::Organization as SupervisorPermissions<u32, T::AccountId>>::is_share_supervisor(organization, share_id.into(), who)
    // }
    fn account_is_member_of_formed_organization(
        org: FormedOrganization,
        who: &T::AccountId,
    ) -> bool {
        match org {
            FormedOrganization::FlatOrg(org_id) => {
                <<T as Trait>::Organization as OrgChecks<u32, T::AccountId>>::check_membership_in_org(org_id, who)
            },
            FormedOrganization::FlatShares(org_id, share_id) => {
                <<T as Trait>::Organization as ShareGroupChecks<u32, ShareID, T::AccountId>>::check_membership_in_share_group(org_id, ShareID::Flat(share_id).into(), who)
            },
            FormedOrganization::WeightedShares(org_id, share_id) => {
                <<T as Trait>::Organization as ShareGroupChecks<u32, ShareID, T::AccountId>>::check_membership_in_share_group(org_id, ShareID::WeightedAtomic(share_id).into(), who)
            },
        }
    }
}

impl<T: Trait> IDIsAvailable<OffChainTreasuryID> for Module<T> {
    fn id_is_available(id: OffChainTreasuryID) -> bool {
        OffChainTreasuryIDs::get(id).is_none()
    }
}

impl<T: Trait> IDIsAvailable<(OffChainTreasuryID, Payment<T::AccountId, BalanceOf<T>>)>
    for Module<T>
{
    fn id_is_available(id: (OffChainTreasuryID, Payment<T::AccountId, BalanceOf<T>>)) -> bool {
        <OffChainTransfers<T>>::get(id.0, id.1).is_none()
    }
}

impl<T: Trait> GenerateUniqueKeyID<(OffChainTreasuryID, Payment<T::AccountId, BalanceOf<T>>)>
    for Module<T>
{
    fn generate_unique_key_id(
        proposed: (OffChainTreasuryID, Payment<T::AccountId, BalanceOf<T>>),
    ) -> (OffChainTreasuryID, Payment<T::AccountId, BalanceOf<T>>) {
        if !Self::id_is_available(proposed.clone()) {
            let mut new_deposit_id = proposed.1.iterate_salt();
            while !Self::id_is_available((proposed.0, new_deposit_id.clone())) {
                new_deposit_id = new_deposit_id.iterate_salt();
            }
            (proposed.0, new_deposit_id)
        } else {
            proposed
        }
    }
}

impl<T: Trait> GenerateUniqueID<OffChainTreasuryID> for Module<T> {
    fn generate_unique_id() -> OffChainTreasuryID {
        let mut current_nonce = OffChainTreasuryIDNonce::get() + 1;
        while !Self::id_is_available(current_nonce) {
            current_nonce += 1;
        }
        OffChainTreasuryIDNonce::put(current_nonce);
        current_nonce
    }
}

impl<T: Trait> SupportedOrganizationShapes for Module<T> {
    type FormedOrgId = FormedOrganization;
}

impl<T: Trait> RegisterOffChainBankAccount for Module<T> {
    type TreasuryId = OffChainTreasuryID; // u32
    fn register_off_chain_bank_account(
        org: Self::FormedOrgId,
    ) -> Result<Self::TreasuryId, DispatchError> {
        let generated_id = Self::generate_unique_id();
        OffChainTreasuryIDs::insert(generated_id, org);
        Ok(generated_id)
    }
}

impl<T: Trait> OffChainBank for Module<T> {
    type Payment = Payment<T::AccountId, BalanceOf<T>>;

    fn sender_claims_payment_sent(id: Self::TreasuryId, payment: Self::Payment) -> Self::Payment {
        let generated_id = Self::generate_unique_key_id((id, payment));
        // sender confirms
        let new_confirmation = PaymentConfirmation::from_sender_claims();
        <OffChainTransfers<T>>::insert(id, generated_id.1.clone(), new_confirmation);
        generated_id.1
    }
    fn recipient_confirms_payment_received(
        id: Self::TreasuryId,
        payment: Self::Payment,
    ) -> DispatchResult {
        let current_confirmation = <OffChainTransfers<T>>::get(id, payment.clone())
            .ok_or(Error::<T>::SenderMustClaimPaymentSentForRecipientToConfirm)?;
        let full_confirm = current_confirmation.put_recipient_confirms();
        <OffChainTransfers<T>>::insert(id, payment, full_confirm);
        Ok(())
    }
    fn check_payment_confirmation(id: Self::TreasuryId, payment: Self::Payment) -> bool {
        if let Some(transfer_state) = <OffChainTransfers<T>>::get(id, payment) {
            return transfer_state.total_confirmation();
        }
        false
    }
}
