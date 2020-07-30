# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- minimal bounty module added
- `OffchainClient` uses ipfs-embed in a structured way (`client/client/src/lib.rs`)

## [0.1.1] - 2020-07-15

- fixed lossy donate module by adding explicit remainder return value
- added percentage thresholds to the vote module which rounds up (instead of down or to the nearest value) (see tests)

## [0.0.6] - 2020-07-07

- generic client library in `client/client` with example usage in `bin/client`
- generic cli library in `client/cli` with example usage in `bin/cli`

### Pallets
- bounty
- bank
- donate
- court
- vote
- org
