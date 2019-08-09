# Minimal Viable Moloch

This module contains three basic implementations of the Moloch DAO mechanism. For ease of comparison, all samples in this repo represent the DAO's logic in a single file.

* [V01: Lots of Objects](./src/v01.rs)
* [V02: Fewer Storage Maps](./src/02.rs)
* [V03: Balancing Readability and Optimization](./src/03.rs)

* need to link everything back to the overarching principles
* here, we are specifically enabling forkability `=>` we want to show how we can implement different patterns for representing organizations and transferring value `=>` there isn't always only one *right* answer
* this is especially important because the DAO design should depend heavily on the use case in question as well as how dynamics change after the DAO is used `=>` this is the basic idea behind the **progress is not monotonic**

**TODO**
* explain why adapting the mechanism design to the DAO activity is important for managing efficiency/costs
* this repo has three distinct tradeoff implementations that are logically equivalent `=>` my favorite is `02`, but `03` is probably the most practical. Explain quickly why that is the case...

## Lots of Objects
> *[`v01.rs`](./src/v01.rs)*


**Note**: choose V01 over V02 in the case that there are a lot of rejected applications (so we don't need to unnecessarily initialize all the `Proposal` and `Election` fields)

## Fewer Storage Maps
> *[`v02.rs`](./src/v02.rs)*

* decrease auxiliary structs and minimize storage maps
* trade for a lot of uninitialized data usage if there are a lot of applications (big cost if there are a lot of applications that are not sponsored as proposals)


## Balancing Readability and Optimization
> *[`v03.rs`](./src/v03.rs)*

If applications are frequently being rejected, create a separate `Application` struct and bring back the three associated maps. This is worth the extra runtime storage if it saves allocating much more runtime storage for all those applications that are never passed! This would look like something between V01 and V02...
