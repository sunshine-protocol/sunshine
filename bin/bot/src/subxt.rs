use crate::error::Error;
use ipfs_embed::Store;
use substrate_subxt::{
    EventSubscription,
    EventsDecoder,
};
use sunshine_bounty_client::bounty::{
    BountyEventsDecoder,
    BountyPaymentExecutedEvent,
    BountyPostedEvent,
    BountyRaiseContributionEvent,
    BountySubmissionPostedEvent,
};
use sunshine_core::ChainClient;
use test_client::{
    Client,
    Runtime,
};

pub async fn bounty_post_subscriber(
    client: &Client<Store>,
) -> Result<EventSubscription<Runtime>, Error> {
    let sub = client.chain_client().subscribe_events().await?;
    let mut decoder =
        EventsDecoder::<Runtime>::new(client.chain_client().metadata().clone());
    decoder.with_bounty();
    let mut sub = EventSubscription::<Runtime>::new(sub, decoder);
    sub.filter_event::<BountyPostedEvent<Runtime>>();
    Ok(sub)
}

pub async fn bounty_contribution_subscriber(
    client: &Client<Store>,
) -> Result<EventSubscription<Runtime>, Error> {
    let sub = client.chain_client().subscribe_events().await?;
    let mut decoder =
        EventsDecoder::<Runtime>::new(client.chain_client().metadata().clone());
    decoder.with_bounty();
    let mut sub = EventSubscription::<Runtime>::new(sub, decoder);
    sub.filter_event::<BountyRaiseContributionEvent<Runtime>>();
    Ok(sub)
}

pub async fn bounty_submission_subscriber(
    client: &Client<Store>,
) -> Result<EventSubscription<Runtime>, Error> {
    let sub = client.chain_client().subscribe_events().await?;
    let mut decoder =
        EventsDecoder::<Runtime>::new(client.chain_client().metadata().clone());
    decoder.with_bounty();
    let mut sub = EventSubscription::<Runtime>::new(sub, decoder);
    sub.filter_event::<BountySubmissionPostedEvent<Runtime>>();
    Ok(sub)
}

pub async fn bounty_approval_subscriber(
    client: &Client<Store>,
) -> Result<EventSubscription<Runtime>, Error> {
    let sub = client.chain_client().subscribe_events().await?;
    let mut decoder =
        EventsDecoder::<Runtime>::new(client.chain_client().metadata().clone());
    decoder.with_bounty();
    let mut sub = EventSubscription::<Runtime>::new(sub, decoder);
    sub.filter_event::<BountyPaymentExecutedEvent<Runtime>>();
    Ok(sub)
}
