#!/bin/sh

git submodule update --init --recursive

MG_MANIFEST_PATH=stratum-message-generator/Cargo.toml

MINING_SUBSCRIBE_TEST_PATH=stratum-message-generator/tests/sv1-subscribe.json
MINING_AUTHORIZE_TEST_PATH=stratum-message-generator/tests/sv1-authorize.json
MINING_CONFIGURE_TEST_PATH=stratum-message-generator/tests/sv1-configure.json

RUST_LOG=info cargo run --manifest-path=$MG_MANIFEST_PATH -- $MINING_SUBSCRIBE_TEST_PATH
RUST_LOG=info cargo run --manifest-path=$MG_MANIFEST_PATH -- $MINING_AUTHORIZE_TEST_PATH
#RUST_LOG=info cargo run --manifest-path=$MG_MANIFEST_PATH -- $MINING_CONFIGURE_TEST_PATH