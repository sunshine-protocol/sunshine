use sp_core::crypto::Pair;
use sp_keyring::AccountKeyring;
use sunshine_client::shares_atomic::*;

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
        .profile(&(&org, &share), &account)
        .await?
        .get_times_reserved();

    let extrinsic_success = xt
        .watch()
        .with_shares_atomic()
        .reserve_shares(org, share, account.clone())
        .await?;
    let event = extrinsic_success.shares_reserved().unwrap()?;

    assert_eq!(reserved + 1, event.reserved);

    println!(
        "Account {:?} reserved {:?} shares with share id {:?} for organization {:?}",
        event.account, event.reserved, event.share, event.org,
    );

    Ok(())
}
