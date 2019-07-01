# SunshineDAO

**Sunshine** is a fund coordination DAO on Substrate inspired by **[Moloch](https://github.com/MolochVentures/moloch)**. The basic idea is to use Moloch's governance mechanism to coordinate membership and raise capital. Thereafter, signalling will strive to be generic and flexible, but I am first prioritizing functionality before abstracting shared behavior. I've identified the following significant tasks 

*Necessary*
- [ ] signalling
    - [ ] members vote (weighted by stake) to fund grants (applications with only the transaction fee)
    - [ ] members vote to accept new members
- [ ] managing sets
    - [ ] changing sets
    - [ ] tracking groups with `Origin` and maps in `decl_storage`
- [ ] lock-in vs fast withdrawal (voting periods) <=> exit mechanisms
- [ ] design criteria

*Optional*
- [ ] (optional/extension) members vote to grant voice in the DAO based on community activism could be useful, but it opens an attack in which the malicious entity increases their relative vote advantage by overpowering the group and voting in someone that agrees with their side on a prominent issue
- [ ] (optional/extension) members vote to remove existing members
- [ ] (optional/extension) using a prediction on an oracle to lock-in a vote on an outcome
- [ ] (optional/extension) creating SubDAOs with a strict subset of the members of the DAO, but the rest of the members weight all their support on someone in the SubDAO