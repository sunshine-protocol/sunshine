# fund

Asset management committee in the context of `sunshine`. Central to any DAO is its ability to flexibly manage its pool of capital.

Each member should be kept as a struct with certain fields. These fields include a projection vector...

**defines which collateral is accepted, how much is accepted, and the criteria for accepting**

## Incentivization and Selection

* use phragmen for delegation

**Melonport Fee Structure**: Fees are rewarded to the manager at certain points during the lifetime of a fund, such as on redemption of shares, and at the end fo a designated reward period. There are currently two types of fees: management and performance fees, and they are tracked by the FeeManager component. Management fees are calculated based on time only, while the amount of performance fees given is determined by share price evolution.

## Current Allocation

* the fund's allocation

## Projected Adjustments

* this forms the basis for the monetary futarchy `=>` just use projections here for participant tallying

## Catalysts

* outlier events that allow the committee to adjust their decisions

# references

* https://www.docs.melonport.com/#melon-protocol-reference -- the only thing missing is *accounting* so this can easily be abstracted into its own module and added with *accounting* to make it just like Melonport

* https://www.fundingnote.com/blog/vc-term-sheets-guide-297 `=>` term sheet

