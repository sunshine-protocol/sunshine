# sunshine

*sunshine* is a governance experiment to demonstrate the flexible and dynamic incentive mechanisms that can be built with Substrate. It was originally inspired by [Moloch](https://github.com/moloch).

## Some Unimplemented Ideas


### Reorder the Order of Defaults During Dilution
Easily glossed over is the dilution safety mechanism in MolochDAO. By definition, the dilution safety limit is designed to control the flow rate of exiting members by setting a hard stop rule if dilution is significant. We can accept a higher successful exit amount if we preemptively use collateral posted by applicants to the dao to fund undiluted exits. Under this new system, applicants post collateral. In optimal scenarios, they benefit from acceptance into the DAO and nothing changes. Worst-case scenarios move the applicant's collateral to the top of the capital stack as the first to go to exiting members before dilution must occur. This payout structure can be further tweaked...For example, maybe we order collateral posted in order of application type such that membership applications are sacrificed before grant application collateral?

The basic idea is to preemptively use collateral to push dilution safety, but there still is a lower dilution limit and the momentum in an overleveraged case can trigger negative feedback loops. Moreover, the nature of this scheme would make investors reluctant to post collateral for applications if exits are being valued above the worth of external applications

### Diverse Signalling and Vesting Payouts

We explore dual elections with different signalling mechanisms. On one hand, pairwise coordination CLR helps determine successful grant applications which are granted according to the associated, accepted `VestingScheduler`. Conversely, membership elections depend on single member votes. We could make this configurable to weigh conviction based on lock-in.

### Modular Fee System

One idea I want to explore is parameterizing the fee mechanism. `fee`s should be an auction-esque module in which the applicant can reduce their per-block rent costs by locking up their capital longer. Additional or scaled benefits should go to promising to lock up their capital for some set amount of time with a penalty fee that can be invoked to exit anyway. Basically, sunshine becomes an availability game, which sounds annoying, but we want to incentivize posting significant collateral. Maybe collateral posted in applications on each side provides the signalling for public whose funding signals are matched?