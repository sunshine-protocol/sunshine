use codec::{
    Codec,
    Decode,
    Encode,
};
use sp_core::TypeId;
use sp_runtime::traits::{
    AtLeast32Bit,
    Zero,
};
use sp_std::prelude::*;

#[derive(
    new, PartialEq, Eq, Clone, Encode, Decode, sp_runtime::RuntimeDebug,
)]
pub struct BankState<
    AccountId,
    OrgId: Codec + PartialEq + Zero + From<u32> + Copy,
    Currency,
> {
    // Registered organization identifier
    org: OrgId,
    // Free for spending
    free: Currency,
    // Reserved for future spending
    reserved: Currency,
    // Layered sudo, representation should be revocable by the group
    controller: Option<AccountId>,
}
