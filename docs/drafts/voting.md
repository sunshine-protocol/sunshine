# Voting

## Sketching the Event Loop

* Each proposal's election process can be modeled as an independent event loop. 

**Vote Event Loop**
* the event loop begins when the proposal is intially made
* each vote added during the voting window can be sent concurrently using channels
* multiple producers (voters)
* a single consumer (the result)

* transitioning to the grace period because of a vote passing a threshold introduces a race condition
* **explain this race condition in documentation**: basically multiple votes can occur at once and trigger the transition, but they won't all be processed and will cause panics because some might check if the vote has passed already and maybe it has...

* so we rely on strict time slots for voting...

**Grace Event Loop**
* each ragequit can be processed as initiating a separate event loop
* no pending supported proposals is verified (lock-in)
* dilution safety is verified
* better ways of dealing with both of these involve "blocking" until the conditions are satisfied to execute the request rather than immediately panicking

## Voting Algorithms and Metagovernance

## Incentives

* look at issues