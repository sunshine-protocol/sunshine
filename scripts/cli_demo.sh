
#!/usr/bin/env bash
# Script to demo the CLI

# 1. build the target CLI
cd ../bin/cli
cargo build --release
cd ../../
# 2. set the keystore password
./target/release/bounty-cli key set
# > requires user input here
# 3. 