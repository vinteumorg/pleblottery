#!/bin/sh

# Set a custom data directory under /tmp
BITCOIN_DATADIR="/tmp/bitcoin_regtest"

# alias to conveniently call bitcoin-cli
alias btc="bitcoin-cli -regtest -datadir=$BITCOIN_DATADIR"