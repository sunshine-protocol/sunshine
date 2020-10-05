# Shared Open Problems

The following open problems are shared by the Web3 space. Our ability to collaboratively fund development on critical infrastructure decides the direction of this technology.

## Substrate Warp Sync

Without warp sync, light clients lack functionality until they are fully synced. Sometimes this can take a really long time (we've experienced anywhere from a few hours to over a day for short-living, low throughput test networks).

OpenEthereum clients receive a snapshot over the network to get the full state at the latest block, and then fill in the blocks between the genesis and the snapshot in the background. The code is [here](https://github.com/openethereum/openethereum/blob/master/ethcore/sync/src/snapshot_sync.rs).

Discussion of Substrate Warp Sync [in this issue](https://github.com/paritytech/substrate/issues/1208).

## Rust-Libp2p Nat Traversal
**[src](https://github.com/w3f/General-Grants-Program/pull/327/files#diff-76eb553547b516da2ea065acc5633ca3R50)**

Nat traversal and firewall traversal is required when peers want to establish a connection to each other. In a traditional server architecture the server gets a public ip like a phone number. Mobile networks or home networks share an ip address, so you can't directly connect to a device that is on a different network. This is done for multiple reasons. Since the ipv4 address space is a 4 byte number, only ~4 million devices can have a unique ip address. Today's number of devices connected to the internet vastly exceeds that amount. But even in ipv6 with a 16 byte address space that allows every device to have a unique ip, the problem of nat traversal will persist. In most cases you don't want arbitrary connections to be opened to arbitrary devices. So in ipv6 firewalls are configured to only allow outgoing connections and reject incoming connections. Techniques used for nat traversal and firewall traversal are and will remain an important part of p2p networks.

### Transport Port Reusability

Transports are assumed by libp2p to have distinct listening and dialing ports. This is an issue when trying to add the quic transport or when using tcp ports with SO_PORTREUSE. Without port reuse, nat traversal becomes impossible without a relay. For the first task, changes to libp2p-core and libp2p-swarm will be made as discussed [here](https://github.com/libp2p/rust-libp2p/issues/1722). The new api will be validated by adding a tcp transport that supports reusing ports and a prototype libp2p-quic crate will be released using this new api. The libp2p-quic crate will live in it's own repo until the rust-libp2p team has the time to review and merge the new transport. Extensive work on the quic transport has already been done by parity employees, but without these api changes it will remain a second class citizen.

### Libp2p Relay

Implement the libp2p relay protocol including tests and examples, showing how to use a third party to establish a connection between two peers that cannot communicate because of a local nat or firewall. Deliverables will be a working libp2p-relay crate. The netsim-embed network simulator we developed will help writing automated tests to verify that it functions correctly.

## Rust Substrate Client

Substrate is written in Rust for a reason; the requirements for blockchain technology align with Rust's dual promise of speed and safety. These requirements extend to the client and make Rust the most practical language for building high-performance, secure clients.

An efficient Rust Substrate client would be able to subscribe to updates only relevant to the client's authorized account(s). Moreover, a well-designed Substrate Rust client would be able to use type metadata to dynamically decode relevant storage data for user display. Although we're not quite there yet, that's the intended direction of [`substrate-subxt`](https://github.com/paritytech/substrate-subxt).

As users of `substrate-subxt`, Sunshine developers contribute upstream often. The `sunshine-bounty` and `sunshine-keybase` repos demonstrate integration of substrate-subxt for Rust client implementations. The [`client/subxt` recipe](./client/subxt.md) contains more details with code examples.