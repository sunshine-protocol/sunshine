## Proposal Event Loop

* applications
* validation/rejection
* execution

### Applications

#### Apply + Sponsor

* for applications (by nonmembers) + sponsorships (by members)

* building a pool of applications, allowing member sponsorship within a defined period

* create some function for gauging the ratio between committed `balance` and `shares_requested`

#### Direct Proposals by Members

* need to think about cases in which an existing member would like to request more shares via a donation

* the application needs to be broken up into two steps (1) the applicant submits to a pool applications

### Application Filtering

### Removal

**why do we batch issuance (proposal execution)?**
* well, it works -- prefer patterns that periodically iterate through the vector instead of doing it too often
* This may be more conducive to the event loop invoked by an off-chain worker

## Brainstorming

* define slashing behavior for voters
* should voters have a fee per vote

* futarchy is the only form of accountable economic security; create a prediction market module (look for simple designs)