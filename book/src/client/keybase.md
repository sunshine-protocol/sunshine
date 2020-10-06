# Keybase

**[`sunshine-keybase`](https://github.com/sunshine-protocol/sunshine-keybase)**

## Chain

The chain module is a reusable abstraction for building private proof of authority chains using ipfs and using substrate to provide authorization and consensus on the current head of the chain. When authoring a block on ipfs a race condition can occur. Due to substrate providing a total order of transactions only one transaction will succeed in updating the head of the chain, the other client will create a new block on the head of the chain and retry the failed operation.

![chain_module.svg](https://draftin.com:443/images/75511?token=ptiW5ycSDqtNQbpH3I24_9YXQQgh2YmbFtDSIT16ZBVaHVtRgQJBeMGmk94Yo3sVGjqJKj86iTmj9y9k6AF2Ujo) 

## Identity

The keybase identity module manages the user's chain that stores the user key, device keys, password and social media accounts using the sunshine chain module. Private data shared between devices is encrypted with the user private key. When a new device is provisioned a key is generated locally on the device, and a provisioning protocol is used to communicate between the new device and the provisioning device.

![keybase-module.svg](https://draftin.com:443/images/75515?token=ZVIuml8B13k3idkoLujuomRsDbSbgUGtzweL7qwj_HNDX8TYlq1iegqpvEnjVjddVdDdle57KVdD7MI7OJES5c8) 

Password changes are stored encrypted in the user chain. When a device receives a block with a password change it reencrypts it's local device key using the new password. This ensures that the user only needs to remember one password.

Social media accounts are linked to a chain account, by submitting a proof in the social media profile and on the user's chain. Other users can find the on chain account on the social media page and verify that they are both controlled by the same cryptographic identity. This allows us to use github usernames as aliases without compromising the decentralized nature or security that blockchains provide. While resolving the social media account to an on chain identity requires the service to be online, already resolved identities are stored locally. This means that even if github is offline, transfers to already verified github accounts can be performed.

Finally the user and team keys will be used in other modules to send encrypted messages, share encrypted files and vote to make decisions.