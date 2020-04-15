use sp_keyring::AccountKeyring;
use sunshine_client::shares_atomic::{reserve_shares, SharesAtomic};
use sunshine_client::system::System;
use sunshine_client::{ClientBuilder, Runtime};

type AccountId = <Runtime as System>::AccountId;
type OrgId = <Runtime as SharesAtomic>::OrgId;
type ShareId = <Runtime as SharesAtomic>::ShareId;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let alice_the_signer = AccountKeyring::Alice.pair();

    let reserves_alices_shares = AccountKeyring::Alice.to_account_id();

    let organization: OrgId = 1u64;
    let share_id: ShareId = 1u64;

    let cli = ClientBuilder::new().build().await?;
    let xt = cli.xt(alice_the_signer, None).await?;
    let extrinsic_success = xt
        .watch()
        .events_decoder(|decoder| {
            // for any primitive event with no type size registered
            decoder.register_type_size::<u64>("OrgId")?;
            decoder.register_type_size::<u64>("ShareId")?;
            decoder.register_type_size::<(u64, u64, u64)>("IdentificationTuple")
        })
        .submit(reserve_shares::<Runtime>(
            organization,
            share_id,
            reserves_alices_shares.clone().into(),
        ))
        .await?;
    let event =  extrinsic_success
        .find_event::<(OrgId, ShareId, AccountId, u32)>("SharesAtomic", "SharesReserved");
    match event {
        Some(Ok((org, share, account, amt))) => println!(
            "Account {:?} reserved {:?} shares with share id {:?} for organization id {:?}",
            account, amt, share, org
        ),
        Some(Err(err)) => println!("Failed to decode code hash: {}", err),
        None => println!("Failed to find SharesAtomic::Reserve Event"),
    }
    Ok(())
}
