#!/usr/bin/env bash
# Script to demo the CLI

cd ../bin/cli
# 1. show all commands supported by the CLI
cargo run -- --help
sleep 3s
# 2. register flat org
cargo run org register-flat-org --help