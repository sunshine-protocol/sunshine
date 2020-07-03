use crate::{
    error::Error,
    Org,
};
use core::fmt::{
    self,
    Debug,
};
use sp_core::crypto::{
    Pair,
    PublicError,
    SecretStringError,
    Ss58Codec,
};
use std::str::FromStr;
use substrate_subxt::{
    sp_core,
    system::System,
};
use thiserror::Error;

#[derive(Clone, Debug)]
pub struct AccountShare(pub String, pub u64);
impl FromStr for AccountShare {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let coords: Vec<&str> = s
            .trim_matches(|p| p == '(' || p == ')')
            .split(',')
            .collect();
        let acc_str = coords[0];
        let share_fromstr =
            coords[1].parse::<u64>().map_err(|_| Error::ParseIntError)?;
        Ok(AccountShare(acc_str.to_string(), share_fromstr))
    }
}

#[derive(Clone)]
pub struct Suri<P: Pair>(pub P::Seed);

impl<P: Pair> Debug for Suri<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "*****")
    }
}

#[derive(Debug, Error)]
#[error("Invalid suri encoded key pair: {0:?}")]
pub struct InvalidSuri(SecretStringError);

impl<P: Pair> FromStr for Suri<P> {
    type Err = InvalidSuri;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let (_, seed) =
            P::from_string_with_seed(string, None).map_err(InvalidSuri)?;
        Ok(Self(seed.unwrap()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ss58<T: System>(pub T::AccountId);

#[derive(Debug, Error)]
#[error("Invalid ss58 encoded public key: {0:?}")]
pub struct InvalidSs58(PublicError);

impl<T: System> FromStr for Ss58<T>
where
    T::AccountId: Ss58Codec,
{
    type Err = InvalidSs58;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            <T::AccountId as Ss58Codec>::from_string(string)
                .map_err(InvalidSs58)?,
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
/// Congruent to `Identifier` type in substrate-identity-cli
pub struct Account<T: Org> {
    pub id: T::AccountId,
}

impl<T: Org> core::fmt::Display for Account<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "{}", self.id)
    }
}

// only for parsing AccountId in particular
impl<T: Org> FromStr for Account<T>
where
    <T as System>::AccountId: Ss58Codec,
{
    type Err = Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        if let Ok(Ss58(account_id)) = Ss58::<T>::from_str(string) {
            Ok(Account { id: account_id })
        } else {
            Err(Error::AccountIdParseFail)
        }
    }
}
