# SunshineDAO

**Sunshine** is a fund coordination DAO on Substrate inspired by **[Moloch](https://github.com/MolochVentures/moloch)**. The basic idea is to use Moloch's governance mechanism to coordinate membership and raise capital. Thereafter, signalling will strive to be generic and flexible, but I am prioritizing functionality before abstracting shared behavior. I've identified the following tasks:

*Necessary*
- [ ] generic signalling
    - [ ] members vote (weighted by stake) to fund grants (applications with only the transaction fee)
    - [ ] members vote to accept new members
- [ ] dynamic membership
    - [ ] use `OnMembershipChanged` trait
    - [ ] tracking groups with a custom `Origin` type
- [ ] lock-in vs fast withdrawal (voting periods) <=> exit mechanisms
- [ ] design criteria writing

*Optional*
- [ ] (optional/extension) members can vote to remove existing members
- [ ] (optional/extension) using a prediction on an oracle to lock-in a vote on an outcome (holographic consensus)
- [ ] (optional/extension) signal delegation with SubDAOs