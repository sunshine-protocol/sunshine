use substrate_subxt::{
    sp_core::Decode,
    Client,
    Event,
    EventSubscription,
    EventsDecoder,
    Runtime,
};
use sunshine_bounty_client::{
    bounty::{
        Bounty,
        BountyClient,
        BountyEventsDecoder,
        BountyPostedEvent,
        MilestoneSubmittedEvent,
    },
    BountyBody,
};

pub async fn bounty_subscriber<
    R: Runtime + Bounty,
    C: BountyClient<R>,
    E: Event<R>,
>(
    client: &C,
    event: E,
) -> Result<EventSubscription<R>, C::Error> {
    let sub = client.subscribe_events().await?;
    let decoder = EventsDecoder::<R>::new(client.metadata().clone());
    let mut sub = EventSubscription::<R>::new(sub, decoder);
    Ok(sub.filter_event::<event>())
}
