# Sunshine Client

At the core, there is a collection of crates that are shared between decoupled, composable sunshine modules. The core crates handle cryptography, codecs, communicating with the dart vm, the substrate light client and ipfs.

Sunshine modules have a substrate runtime component, a client component and a cli and ffi interface.

![architecture.svg](https://draftin.com:443/images/75510?token=FlEvvHLf4Y-96u_aC6FWYchEnEu_4GRR6XihvxbWZXaWV3aPOc066yQ1IcqmtbzMz9txZL1l-hW3re7RYBOu0aM)

The node, runtime, client and ffi is composed of sunshine modules and lives in the [sunshine repo](https://github.com/sunshine-protocol/sunshine). [Sunscreen](https://github.com/sunshine-protocol/sunscreen) is our android and ios flutter ui that uses the sunshine ffi to create a mobile first user experience.

* [Ipfs-Embed](embed.md)
* [Keybase](keybase.md)
* [Substrate-Subxt](subxt.md)