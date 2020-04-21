use sp_core::crypto::Pair;
use sp_keyring::AccountKeyring;
use sunshine_client::shares_atomic::*;
use sunshine_client::system::System;
use sunshine_client::Runtime;

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let signer = AccountKeyring::Eve.pair();
    let client = sunshine_client::build_client().await?;
    let xt = client.xt(signer.clone(), None).await?;

    let org = 1;
    let share = 1;
    let account = signer.public().into();
    let reserved = client
        .profile(&org, &share, &account)
        .await?
        .get_times_reserved();

    let extrinsic_success = xt
        .watch()
        .with_shares_atomic()
        .submit(reserve_shares::<Runtime>(org, share, account.clone()))
        .await?;
    let res = extrinsic_success.shares_reserved().unwrap()?;

    assert_eq!((org, share, account, reserved + 1), res);

    let (org, share, account, reserved) = res;
    println!(
        "Account {:?} reserved {:?} shares with share id {:?} for organization {:?}",
        account, reserved, share, org,
    );

    Ok(())
}
