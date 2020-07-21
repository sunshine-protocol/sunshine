use crate::error::Error;
use ipfs_embed::Store;
use substrate_subxt::{
    sp_core::Decode,
    Event,
    EventSubscription,
    EventsDecoder,
};
use sunshine_bounty_client::{
    bounty::{
        BountyClient,
        BountyEventsDecoder,
        BountyPostedEvent,
        MilestoneSubmittedEvent,
    },
    BountyBody,
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

pub async fn milestone_submission_subscriber(
    client: &Client<Store>,
) -> Result<EventSubscription<Runtime>, Error> {
    let sub = client.chain_client().subscribe_events().await?;
    let mut decoder =
        EventsDecoder::<Runtime>::new(client.chain_client().metadata().clone());
    decoder.with_bounty();
    let mut sub = EventSubscription::<Runtime>::new(sub, decoder);
    sub.filter_event::<MilestoneSubmittedEvent<Runtime>>();
    Ok(sub)
}
