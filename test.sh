#!/bin/bash
# Simplified test script - Node 2 and Send Command only

echo "====================================================="
echo " CipherStream P2P Direct Connection Test (Send Cmd)"
echo "====================================================="

# Create test file
echo "Creating test file..."
echo "Direct connection test data." > test_file.txt
echo 

# Clean up previous logs/files
rm -f node1.log node2.log file_transfer.log logs/cipherstream_node.*

# Set detailed logging
export RUST_LOG="info,cipherstream=debug,libp2p_swarm=debug,libp2p_tcp=debug,libp2p_noise=trace,libp2p_yamux=trace"
echo "🔧 Set RUST_LOG=$RUST_LOG"
echo

# Build the application
echo "Building CipherStream..."
cargo build
echo

# Start Node 2 in the background
echo "====================================================="
echo "Starting node 2 on port 9001..."
echo "====================================================="
nohup cargo run -- start --port 9001 > node2.log 2>&1 &
NODE2_PID=$!
sleep 15 # Give it time to start and log its Peer ID

# Get Node 2 Peer ID
PEER2=$(grep -m 1 "Peer ID: " logs/cipherstream_node.* | sed 's/.*Peer ID: //g' | tail -n 1)
if [ -z "$PEER2" ]; then
    PEER2=$(grep -m 1 "Peer ID: " node2.log | sed 's/.*Peer ID: //') # Fallback
fi

if [ -z "$PEER2" ]; then
    echo "❌ Failed to get Peer ID for Node 2."
    kill $NODE2_PID 2>/dev/null
    exit 1
fi
echo "Target Node 2 Peer ID: $PEER2"
echo

# Attempt to send file directly to Node 2
echo "====================================================="
echo "Attempting to send file directly to Node 2 ($PEER2)..."
echo "====================================================="
cargo run -- send --peer $PEER2 --file test_file.txt > file_transfer.log 2>&1 

# Wait for the file transfer command to potentially finish
echo "Waiting for send command to complete (65 seconds)..."
sleep 65
echo

# Display the file transfer output log
echo "File transfer log (file_transfer.log):"
echo "====================================================="
cat file_transfer.log
echo "====================================================="
echo

# Check logs for connection status
echo "Checking for connection events in transfer log..."
if grep -q "Connection established with TARGET peer: $PEER2" file_transfer.log; then
    echo "✅ Temporary node connected to Node 2"
elif grep -q "OutgoingConnectionError.*TARGET peer $PEER2" file_transfer.log; then
    echo "❌ Temporary node failed to connect to Node 2 (OutgoingConnectionError logged)"
elif grep -q "OutgoingConnectionError.*Transport.*Timeout" file_transfer.log; then
    echo "❌ Temporary node failed to connect to Node 2 (Timeout logged)"
elif grep -q "Operation timeout reached" file_transfer.log; then
    echo "❌ Send command timed out before connection/completion."
else
    echo "❓ Connection status unclear from temporary node log."
fi
echo

# Stop Node 2
echo "====================================================="
echo "Stopping node 2..."
echo "====================================================="
kill $NODE2_PID
wait $NODE2_PID 2>/dev/null

echo "Test completed."