# Metagovernance
> *researching now*

* should use modeling for this I think...needs more research rather than a short-term implementation

* lots of metadata requires for all items, but we need to limit storage overhead so I think that's where clever handling comes into play...

* governing all of these parameters in a consummable way
* explaining every choice and how to make it differently
* we need to audit every mechanism design decision

* Transaction Fees and How They Flow in a Closed System
* Dilution Bound?

* Shawn's article on `Sudo`

## Protocol Engineering vs Mechanism Design (cached here?)

* make a distinction between protocol and application level; maybe describe how Bitcoin works...it is an engineered Cantillon effect; modern block reward proposals have similar intentions, but the real problem with this at the application level is that it hides a tax on user's funds...
* segway into the UTXO example with this...

* maybe make the title more specific...

## Gav's Notes (from Sub0)
> rules => decisions => accountability

Consider rules within a system vs over a system

**Traditional Model**
* leads to fragmentation
* centralization of decision making
* risk of stagnancy, fragmentation, silent coup d'etats
* leads to vague and dumb processes, personality-driven decision making
* no means to ensure ccountability and transparency

**The Metaprotocol Model**
* the underlying protocol never changes
* the nebulous crowd of decision puts its decisions into the protocol => the governance rules move the protocol along in a transparent and deterministic process
* the protocol was moved according to the underlying rules
* a way to manage the contention => forcing some degree of compromise
* rules must be formalized
* processes are autonomously enforced/executed
* allows for dynamic consensus that tunes the speed of evolution
* risk of volatility and collapse
* can always "fall back" into traditional model in dire straits (offline manual consensus)

**Jam's Paradox: every (`closed`) system converges to capture**
* formal governance => systemic bias => capture
* informal governance => converge to centralization of decision making => capture

So how do we build systems that aren't susceptible to *capture*? Build systems with dynamic stakeholders set and coordinated exit in the event of capture
* how do we know when capture has occurred?

We need to draw a distinction between formalized and static structures, and formalized and dynamic structures; the latter hasn't really been explored as much...
* look at how the US government does things; we basically require higher thresholds to change existing rules
* but could we change the voting rules themselves? This is an attack vector that could lead to capture...

| formal | informal |
| static | dynamic  |

Too dynamic => risk of volatility and collapse