use crate::{
    error::{
        Error,
        Result,
    },
    srml::{
        bank::*,
        bounty::*,
        donate::*,
        org::*,
        vote::*,
    },
};
use async_std::sync::{
    Mutex,
    RwLock,
};
use codec::Decode;
use core::marker::PhantomData;
use ipld_block_builder::{
    BlockBuilder,
    Codec,
};
use keystore::{
    DeviceKey,
    KeyStore,
    Password,
};
use libipld::store::Store;
use substrate_subxt::{
    sp_core::crypto::{
        Pair,
        Ss58Codec,
    },
    sp_runtime::traits::{
        IdentifyAccount,
        SignedExtension,
        Verify,
    },
    system::System,
    PairSigner,
    Runtime,
    SignedExtra,
};
use util::{
    court::ResolutionMetadata,
    vote::VoterView,
};

pub struct Client<T, P, I>
where
    T: Runtime + Org + Vote + Donate + Bank + Bounty,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    P: Pair,
    I: Store,
{
    _marker: PhantomData<P>,
    keystore: RwLock<KeyStore>,
    subxt: substrate_subxt::Client<T>,
    pub cache: Mutex<BlockBuilder<I, Codec>>,
}

impl<T, P, I> Client<T, P, I>
where
    T: Runtime + Org + Vote + Donate + Bank + Bounty,
    <T as System>::AccountId: Into<<T as System>::Address> + Ss58Codec,
    T::Signature: Decode + From<P::Signature>,
    <T::Signature as Verify>::Signer:
        From<P::Public> + IdentifyAccount<AccountId = <T as System>::AccountId>,
    <<T::Extra as SignedExtra<T>>::Extra as SignedExtension>::AdditionalSigned:
        Send + Sync,
    P: Pair,
    <P as Pair>::Public: Into<<T as System>::AccountId>,
    <P as Pair>::Seed: From<[u8; 32]>,
    I: Store,
{
    pub fn new(
        keystore: KeyStore,
        subxt: substrate_subxt::Client<T>,
        store: I,
    ) -> Self {
        Self {
            _marker: PhantomData,
            keystore: RwLock::new(keystore),
            subxt,
            cache: Mutex::new(BlockBuilder::new(store, Codec::new())),
        }
    }
    /// Set device key, directly from substrate-identity to use with keystore
    pub async fn has_device_key(&self) -> bool {
        self.keystore.read().await.is_initialized().await
    }
    /// Set device key, directly from substrate-identity to use with keystore
    pub async fn set_device_key(
        &self,
        dk: &DeviceKey,
        password: &Password,
        force: bool,
    ) -> Result<<T as System>::AccountId> {
        if self.keystore.read().await.is_initialized().await && !force {
            return Err(Error::KeystoreInitialized)
        }
        let pair = P::from_seed(&P::Seed::from(*dk.expose_secret()));
        self.keystore
            .write()
            .await
            .initialize(&dk, &password)
            .await?;
        Ok(pair.public().into())
    }
    /// Returns a signer for alice
    pub async fn signer(&self) -> Result<PairSigner<T, P>> {
        // fetch device key from disk every time to make sure account is unlocked.
        let dk = self.keystore.read().await.device_key().await?;
        Ok(PairSigner::new(P::from_seed(&P::Seed::from(
            *dk.expose_secret(),
        ))))
    }
    // org logic
    pub async fn register_flat_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::IpfsReference,
        members: &[<T as System>::AccountId],
    ) -> Result<NewFlatOrganizationRegisteredEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .register_flat_org_and_watch(
                &signer,
                sudo,
                parent_org,
                constitution,
                members,
            )
            .await?
            .new_flat_organization_registered()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn register_weighted_org(
        &self,
        sudo: Option<<T as System>::AccountId>,
        parent_org: Option<<T as Org>::OrgId>,
        constitution: <T as Org>::IpfsReference,
        weighted_members: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<NewWeightedOrganizationRegisteredEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .register_weighted_org_and_watch(
                &signer,
                sudo,
                parent_org,
                constitution,
                weighted_members,
            )
            .await?
            .new_weighted_organization_registered()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesIssuedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .issue_shares_and_watch(&signer, organization, &who, shares)
            .await?
            .shares_issued()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        who: <T as System>::AccountId,
        shares: <T as Org>::Shares,
    ) -> Result<SharesBurnedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .burn_shares_and_watch(&signer, organization, &who, shares)
            .await?
            .shares_burned()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn batch_issue_shares(
        &self,
        organization: <T as Org>::OrgId,
        new_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchIssuedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .batch_issue_shares_and_watch(&signer, organization, new_accounts)
            .await?
            .shares_batch_issued()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn batch_burn_shares(
        &self,
        organization: <T as Org>::OrgId,
        old_accounts: &[(<T as System>::AccountId, <T as Org>::Shares)],
    ) -> Result<SharesBatchBurnedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .batch_burn_shares_and_watch(&signer, organization, old_accounts)
            .await?
            .shares_batch_burned()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn reserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesReservedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .reserve_shares_and_watch(&signer, org, who)
            .await?
            .shares_reserved()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn unreserve_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnReservedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .unreserve_shares_and_watch(&signer, org, who)
            .await?
            .shares_un_reserved()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn lock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesLockedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .lock_shares_and_watch(&signer, org, who)
            .await?
            .shares_locked()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn unlock_shares(
        &self,
        org: <T as Org>::OrgId,
        who: &<T as System>::AccountId,
    ) -> Result<SharesUnlockedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .unlock_shares_and_watch(&signer, org, who)
            .await?
            .shares_unlocked()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    // vote logic
    pub async fn create_threshold_approval_vote(
        &self,
        topic: Option<<T as Org>::IpfsReference>,
        organization: T::OrgId,
        support_requirement: T::Signal,
        turnout_requirement: Option<T::Signal>,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .create_threshold_approval_vote_and_watch(
                &signer,
                topic,
                organization,
                support_requirement,
                turnout_requirement,
                duration,
            )
            .await?
            .new_vote_started()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn create_unanimous_consent_approval_vote(
        &self,
        topic: Option<<T as Org>::IpfsReference>,
        organization: T::OrgId,
        duration: Option<<T as System>::BlockNumber>,
    ) -> Result<NewVoteStartedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .create_unanimous_consent_vote_and_watch(
                &signer,
                topic,
                organization,
                duration,
            )
            .await?
            .new_vote_started()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn submit_vote(
        &self,
        vote_id: <T as Vote>::VoteId,
        direction: VoterView,
        justification: Option<<T as Org>::IpfsReference>,
    ) -> Result<VotedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .submit_vote_and_watch(&signer, vote_id, direction, justification)
            .await?
            .voted()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    // donate logic
    pub async fn make_prop_donation_with_fee(
        &self,
        org: <T as Org>::OrgId,
        amt: DonateBalanceOf<T>,
    ) -> Result<DonationExecutedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .make_prop_donation_with_fee_and_watch(&signer, org, amt)
            .await?
            .donation_executed()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn make_prop_donation_without_fee(
        &self,
        org: <T as Org>::OrgId,
        amt: DonateBalanceOf<T>,
    ) -> Result<DonationExecutedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .make_prop_donation_without_fee_and_watch(&signer, org, amt)
            .await?
            .donation_executed()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    // bank logic
    pub async fn open_org_bank_account(
        &self,
        seed: BalanceOf<T>,
        hosting_org: <T as Org>::OrgId,
        bank_operator: Option<<T as System>::AccountId>,
    ) -> Result<OrgBankAccountOpenedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .open_org_bank_account_and_watch(
                &signer,
                seed,
                hosting_org,
                bank_operator,
            )
            .await?
            .org_bank_account_opened()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    // bounty logic
    pub async fn account_posts_bounty(
        &self,
        description: <T as Org>::IpfsReference,
        amount_reserved_for_bounty: BalanceOf<T>,
        acceptance_committee: ResolutionMetadata<
            <T as Org>::OrgId,
            <T as Vote>::Signal,
            <T as System>::BlockNumber,
        >,
        supervision_committee: Option<
            ResolutionMetadata<
                <T as Org>::OrgId,
                <T as Vote>::Signal,
                <T as System>::BlockNumber,
            >,
        >,
    ) -> Result<BountyPostedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .account_posts_bounty_and_watch(
                &signer,
                description,
                amount_reserved_for_bounty,
                acceptance_committee,
                supervision_committee,
            )
            .await?
            .bounty_posted()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn account_applies_for_bounty(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        description: <T as Org>::IpfsReference,
        total_amount: BalanceOf<T>,
    ) -> Result<BountyApplicationSubmittedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .account_applies_for_bounty_and_watch(
                &signer,
                bounty_id,
                description,
                total_amount,
            )
            .await?
            .bounty_application_submitted()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn account_triggers_application_review(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        new_grant_app_id: <T as Bounty>::BountyId,
    ) -> Result<ApplicationReviewTriggeredEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .account_triggers_application_review_and_watch(
                &signer,
                bounty_id,
                new_grant_app_id,
            )
            .await?
            .application_review_triggered()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn account_sudo_approves_application(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
    ) -> Result<SudoApprovedApplicationEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .account_sudo_approves_application_and_watch(
                &signer,
                bounty_id,
                application_id,
            )
            .await?
            .sudo_approved_application()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn poll_application(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
    ) -> Result<ApplicationPolledEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .poll_application_and_watch(&signer, bounty_id, application_id)
            .await?
            .application_polled()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn submit_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        application_id: <T as Bounty>::BountyId,
        submission_reference: <T as Org>::IpfsReference,
        amount_requested: BalanceOf<T>,
    ) -> Result<MilestoneSubmittedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .submit_milestone_and_watch(
                &signer,
                bounty_id,
                application_id,
                submission_reference,
                amount_requested,
            )
            .await?
            .milestone_submitted()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn trigger_milestone_review(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestoneReviewTriggeredEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .trigger_milestone_review_and_watch(
                &signer,
                bounty_id,
                milestone_id,
            )
            .await?
            .milestone_review_triggered()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn sudo_approves_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestoneSudoApprovedEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .sudo_approves_milestone_and_watch(&signer, bounty_id, milestone_id)
            .await?
            .milestone_sudo_approved()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
    pub async fn poll_milestone(
        &self,
        bounty_id: <T as Bounty>::BountyId,
        milestone_id: <T as Bounty>::BountyId,
    ) -> Result<MilestonePolledEvent<T>> {
        let signer = self.signer().await?;
        self.subxt
            .clone()
            .poll_milestone_and_watch(&signer, bounty_id, milestone_id)
            .await?
            .milestone_polled()
            .map_err(substrate_subxt::Error::Codec)?
            .ok_or(Error::EventNotFound)
    }
}
