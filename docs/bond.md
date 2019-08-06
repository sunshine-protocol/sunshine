# Bonding Collateral

> overall timeline diagram

We could bond additional capital for the `vote_bond` or bond 

## Application Bonds

* bond the applicant

## Proposal, Bond Shares

> diagram with proposal initiation, vote, abort, grace `=>` disposal (reference event loop)

* should definitely switch to just bonding the applier in the `do_propose` method, the proposer must commit a higher than average portion of his shares to sponsor the proposal. They must support the proposal throughout the vote and grace windows. They get a higher amount of the overall reward
* we should not bond the proposer with actual capital -- this is annoying for interaction `=>` we want to limit balance transfers to entry and exit (**mental transaction costs**); instead, bond votes by shares

## Vote, Bond Shares

> diagram with vote window

If you are on the losings side, you can take your vote out during the GraceWindow to thereafter `deregsiter` and `burn` shares to exit. If you are on the winning side, you leave a bit later `=>` this effects the dilution benefits
* need to run some scenarios to study the dilution effects `=>` could code in basic python

This is called lock-in and instant withdrawal.