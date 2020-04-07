use sp_keyring::AccountKeyring;
use substrate_subxt::{self, balances::Balances, system::System, ExtrinsicSuccess};
// mod vote_yesno;
// use vote_yesno::{self, *};
mod shares_atomic;
use shares_atomic::SharesAtomic;
use sp_runtime::{
    generic::Header,
    traits::{BlakeTwo256, IdentifyAccount, Verify},
    MultiSignature, OpaqueExtrinsic, Permill,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Runtime;

impl System for Runtime {
    type Index = u32;
    type BlockNumber = u32;
    type Hash = sp_core::H256;
    type Hashing = BlakeTwo256;
    type AccountId = <<MultiSignature as Verify>::Signer as IdentifyAccount>::AccountId;
    type Address = ();
    type Header = Header<Self::BlockNumber, BlakeTwo256>;
    type Extrinsic = OpaqueExtrinsic;
    type AccountData = AccountData<<Self as Balances>::Balance>;
}

impl Balances for Runtime {
    type Balance = u128;
}

impl SharesAtomic for Runtime {
    type OrgId = u32;
    type ShareId = u32;
}

type AccountId = <Runtime as System>::AccountId;
type OrgId = <Runtime as SharesAtomic>::OrgId;
type ShareId = <Runtime as SharesAtomic>::ShareId;

// impl VoteYesNo for SunshineRuntime {
//     type VoteId = u64;
//     type ShareData = SharesAtomic;
// }

fn main() {
    let result: Result<ExtrinsicSuccess<_>, Box<dyn std::error::Error + 'static>> =
        async_std::task::block_on(async move {
            env_logger::init();

            let signer = AccountKeyring::Alice.pair();

            let checked_account = AccountKeyring::Bob.to_account_id();

            let cli = substrate_subxt::ClientBuilder::<Runtime>::new()
                .build()
                .await?;
            let xt = cli.xt(signer, None).await?;
            let xt_result = xt
                .watch()
                .events_decoder(|decoder| {
                    // for any primitive event with no type size registered
                    decoder.register_type_size::<(u64, u64)>("IdentificationTuple")
                })
                .submit(shares_atomic::reserve::<Runtime>(
                    1u32,
                    1u32,
                    checked_account.clone().into(),
                ))
                .await?;
            Ok(xt_result)
        });
    match result {
        Ok(extrinsic_success) => {
            match extrinsic_success
                .find_event::<(OrgId, ShareId, AccountId)>("SharesAtomic", "Reserve")
            {
                Some(Ok((org, share, account))) => println!(
                    "Account {:?} reserved id number {:?} shares for id number {:?} organization",
                    account, share, org
                ),
                Some(Err(err)) => println!("Failed to decode code hash: {}", err),
                None => println!("Failed to find SharesAtomic::Reserve Event"),
            }
        }
        Err(err) => println!("Error: {:?}", err),
    }
}
