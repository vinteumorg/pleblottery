#!/bin/sh

# Get the directory of the current script
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source env.sh
source "$SCRIPT_DIR/env.sh"

# Stop any running bitcoind instance (if any)
echo "Stopping any running bitcoind instance..."
bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" stop 2>/dev/null

# Wait for bitcoind to stop
sleep 3

# Remove the custom regtest directory to clear any existing data
echo "Cleaning up regtest data directory at $BITCOIN_DATADIR..."
rm -rf "$BITCOIN_DATADIR/regtest"

# Ensure the data directory exists
if [ ! -d "$BITCOIN_DATADIR" ]; then
    echo "Creating data directory at $BITCOIN_DATADIR..."
    mkdir -p "$BITCOIN_DATADIR"
fi

# create bitcoin.conf
cat << EOF > $BITCOIN_DATADIR/bitcoin.conf
[regtest]
server=1
rpcuser=username
rpcpassword=username
EOF

# Start bitcoind in regtest mode with the custom data directory
echo "Starting bitcoind in regtest mode with custom data directory..."
bitcoind -regtest -datadir="$BITCOIN_DATADIR" -daemon

# Wait for bitcoind to fully start
echo "Waiting for bitcoind to start..."
sleep 5

# Create a new address to mine to
echo "Creating a new address to receive mining rewards..."
bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" createwallet regtest
MINING_ADDRESS=$(bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" getnewaddress)

# Generate 16 blocks to the new address
echo "Generating 16 blocks to $MINING_ADDRESS..."
bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" generatetoaddress 16 "$MINING_ADDRESS"
echo "16 blocks have been generated."

echo "calling getblockchaininfo:"
bitcoin-cli -regtest -datadir="$BITCOIN_DATADIR" getblockchaininfo

echo "Done."
