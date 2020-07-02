use clap::Clap;
use std::{
    path::PathBuf,
    str::FromStr,
};
use substrate_subxt::{
    sp_core::{
        crypto::Ss58Codec,
        sr25519,
        Pair,
    },
    sp_runtime::{
        traits::{
            IdentifyAccount,
            Verify,
        },
        MultiSignature,
    },
};

pub type AccountId =
    <<MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;
pub type OrgId = u64;
pub type Shares = u64;

#[derive(Clone, Debug, Clap)]
pub struct Opts {
    #[clap(subcommand)]
    pub cmd: SubCommand,
    #[clap(short = "p", long = "path")]
    pub path: Option<PathBuf>,
}

#[derive(Clone, Debug, Clap)]
pub enum SubCommand {
    Key(KeyCommand),
    Wallet(WalletCommand),
    Run,
}

#[derive(Clone, Debug, Clap)]
pub struct KeyCommand {
    #[clap(subcommand)]
    pub cmd: KeySubCommand,
}

#[derive(Clone, Debug, Clap)]
pub enum KeySubCommand {
    Set(KeySetCommand),
    Unlock,
    Lock,
}

#[derive(Clone, Debug, Clap)]
pub struct WalletCommand {
    #[clap(subcommand)]
    pub cmd: WalletSubCommand,
}

#[derive(Clone, Debug, Clap)]
pub enum WalletSubCommand {
    IssueShares(WalletIssueSharesCommand),
    ReserveShares(WalletReserveSharesCommand),
    LockShares(WalletLockSharesCommand),
}

#[derive(Clone, Debug, Clap)]
pub struct WalletIssueSharesCommand {
    pub organization: Identifier,
    pub who: Identifier,
    pub shares: Identifier,
}

#[derive(Clone, Debug, Clap)]
pub struct WalletReserveSharesCommand {
    pub organization: Identifier,
    pub who: Identifier,
}

#[derive(Clone, Debug, Clap)]
pub struct WalletLockSharesCommand {
    pub organization: Identifier,
    pub who: Identifier,
}

#[derive(Clone)]
pub struct Suri(pub [u8; 32]);

impl core::fmt::Debug for Suri {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        write!(f, "*****")
    }
}

impl FromStr for Suri {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let (_, seed) = sr25519::Pair::from_string_with_seed(string, None)
            .map_err(|_| Error::InvalidSuri)?;
        Ok(Self(seed.unwrap()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ss58(pub AccountId);

impl FromStr for Ss58 {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        Ok(Self(
            AccountId::from_string(string).map_err(|_| Error::InvalidSs58)?,
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Identifier {
    Org(OrgId),
    Account(AccountId),
    Shares(Shares),
}

impl Identifier {
    pub fn into_account(self) -> Option<AccountId> {
        match self {
            Identifier::Account(acc) => Some(acc),
            _ => None,
        }
    }
}

use core::convert::TryInto;
impl TryInto<u64> for Identifier {
    type Error = Error;
    fn try_into(self) -> Result<u64, Self::Error> {
        match self {
            Identifier::Org(org_id) => Ok(org_id),
            Identifier::Shares(shares) => Ok(shares),
            _ => Err(Error::IdentifierConversionFailed),
        }
    }
}

impl core::fmt::Display for Identifier {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::Org(org_id) => write!(f, "{}", org_id),
            Self::Account(account_id) => write!(f, "{}", account_id),
            Self::Shares(shares) => write!(f, "{}", shares),
        }
    }
}

impl FromStr for Identifier {
    type Err = Error;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        if let Ok(org_id) = OrgId::from_str(string) {
            Ok(Self::Org(org_id))
        } else if let Ok(shares) = Shares::from_str(string) {
            Ok(Self::Shares(shares))
        } else if let Ok(Ss58(account_id)) = Ss58::from_str(string) {
            Ok(Self::Account(account_id))
        } else {
            Err(Error::UnparsedIdentifier)
        }
    }
}
