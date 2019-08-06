# Storage

We want to establish the following objectives:
1. ease of unit testing to demonstrate best practice
2. minimize calls to storage through optimal assignment of fields to structs -- NOTE instances in which two map calls are necessary and determine if the structs out to be combined `=>` eventually, benchmark this behavior?

## Configuration in Storage

The use of this `config.build` from `srml/balances` is useful for bootstrapping a storage state from no state.