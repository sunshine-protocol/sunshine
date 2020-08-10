//! Structured call data for `vote`
use crate::vote::Threshold;
use codec::{
    Decode,
    Encode,
};
pub use sp_core::Hasher;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

#[derive(new, PartialEq, Eq, Clone, Copy, Encode, Decode, RuntimeDebug)]
pub struct VoteCall<Org, VoteThreshold, BlockNumber> {
    pub org: Org,
    pub threshold: VoteThreshold,
    pub duration: Option<BlockNumber>,
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub enum VoteMetadata<Org, Signal, Permill, BlockNumber> {
    Signal(VoteCall<Org, Threshold<Signal>, BlockNumber>),
    Percentage(VoteCall<Org, Threshold<Permill>, BlockNumber>),
}

impl<Org: Copy, Signal: Copy, Permill: Copy, BlockNumber: Copy>
    VoteMetadata<Org, Signal, Permill, BlockNumber>
{
    pub fn org(&self) -> Org {
        match self {
            VoteMetadata::Signal(v) => v.org,
            VoteMetadata::Percentage(v) => v.org,
        }
    }
    pub fn duration(&self) -> Option<BlockNumber> {
        match self {
            VoteMetadata::Signal(v) => v.duration,
            VoteMetadata::Percentage(v) => v.duration,
        }
    }
}

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct ResolutionMetadata<AccountId, VoteMetadata> {
    sudo: Option<AccountId>,
    vote: Option<VoteMetadata>,
}

impl<AccountId: Clone + PartialEq, VoteMetadata: Clone>
    ResolutionMetadata<AccountId, VoteMetadata>
{
    pub fn new(
        sudo: Option<AccountId>,
        vote: Option<VoteMetadata>,
    ) -> Option<Self> {
        if sudo.is_none() && vote.is_none() {
            // both fields == None is not a valid configuration
            None
        } else {
            Some(Self { sudo, vote })
        }
    }
    pub fn sudo(&self) -> Option<AccountId> {
        self.sudo.clone()
    }
    pub fn is_sudo(&self, who: &AccountId) -> bool {
        if let Some(s) = self.sudo() {
            &s == who
        } else {
            false
        }
    }
    pub fn vote(&self) -> Option<VoteMetadata> {
        self.vote.clone()
    }
}
