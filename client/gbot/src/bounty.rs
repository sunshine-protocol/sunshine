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
use substrate_subxt::{
    Runtime,
    Client,
    sp_core::Decode,
    EventSubscription,
    EventsDecoder,
};

pub async fn bounty_post_subscriber<R: Runtime + Bounty, C: BountyClient<R>>(
    client: &C
) -> Result<EventSubscription<R>, C::Error> 
where
    <R as Bounty>::BountyPost: From<BountyBody>,
{
    let sub = client.subscribe_events().await?;
    let mut decoder = EventsDecoder::<R>::new(client.metadata().clone());
    decoder.with_bounty();
    let mut sub = EventSubscription::<R>::new(sub, decoder);
    Ok(sub.filter_event::<BountyPostedEvent<_>>())
}

pub async fn milestone_submission_subscriber<R: Runtime + Bounty, C: BountyClient<R>>(
    client: &C
) -> Result<EventSubscription<R>, C::Error>
where
    <R as Bounty>::MilestoneSubmission: From<BountyBody>,
{
    let sub = client.subscribe_events().await?;
    let mut decoder = EventsDecoder::<R>::new(client.metadata().clone());
    decoder.with_bounty();
    let mut sub = EventSubscription::<R>::new(sub, decoder);
    Ok(sub.filter_event::<MilestoneSubmittedEvent<_>>())
}