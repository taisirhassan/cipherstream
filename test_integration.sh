#!/bin/bash

# This script runs a real integration test with two separate nodes
# It uses dynamic port allocation to avoid port conflicts

# Create a temporary directory for test files and data
TEST_DIR=".test_integration"
mkdir -p $TEST_DIR

# Generate test file with random content 
dd if=/dev/urandom of=$TEST_DIR/test_file.bin bs=1M count=1 2>/dev/null
echo "Created 1MB test file: $TEST_DIR/test_file.bin"

# Generate random ports to avoid port conflicts
PORT1=$((9000 + RANDOM % 1000))
PORT2=$((9000 + RANDOM % 1000))
echo "Using ports: $PORT1 (receiver) and $PORT2 (sender)"

# Start receiver node
echo "Starting receiver node on port $PORT1..."
cargo run --quiet -- start --port $PORT1 --data-dir "$TEST_DIR/node1" > $TEST_DIR/node1.log 2>&1 &
RECEIVER_PID=$!

# Sleep to ensure receiver is fully started
sleep 5

# Extract peer ID from logs
RECEIVER_PEER_ID=$(grep -oE "🆔 Local peer id: [0-9a-zA-Z]+" $TEST_DIR/node1.log | cut -d' ' -f4)
if [ -z "$RECEIVER_PEER_ID" ]; then
    echo "Failed to get peer ID from receiver node"
    kill $RECEIVER_PID
    exit 1
fi

echo "Receiver node running with peer ID: $RECEIVER_PEER_ID"

# Start sender node
echo "Starting sender node (temp) to send file..."
(cargo run --quiet -- send --peer $RECEIVER_PEER_ID --file "$TEST_DIR/test_file.bin" --encrypt > $TEST_DIR/sender.log 2>&1) &
SENDER_PID=$!

# Wait for file transfer to complete (with timeout)
echo "Waiting for transfer to complete (max 30 seconds)..."
TIMEOUT=30
while [ $TIMEOUT -gt 0 ]; do
    if grep -q "File transfer completed" $TEST_DIR/sender.log; then
        echo "✅ File transfer reported as successful!"
        break
    fi
    
    if grep -q "Failed to send file" $TEST_DIR/sender.log; then
        echo "❌ File transfer reported as failed!"
        break
    fi
    
    sleep 1
    TIMEOUT=$((TIMEOUT - 1))
done

if [ $TIMEOUT -eq 0 ]; then
    echo "❌ Timeout waiting for file transfer to complete"
fi

# Verify files match if they exist
RECEIVED_FILE=$(find $TEST_DIR/node1 -name "test_file.bin" -type f)
if [ -n "$RECEIVED_FILE" ]; then
    echo "Found received file: $RECEIVED_FILE"
    
    # Compare files
    if cmp -s "$TEST_DIR/test_file.bin" "$RECEIVED_FILE"; then
        echo "✅ Files match! Transfer was successful."
    else
        echo "❌ Files don't match. Transfer was corrupted."
    fi
else
    echo "❌ Received file not found."
fi

# Clean up
echo "Cleaning up..."
kill $RECEIVER_PID $SENDER_PID 2>/dev/null

# Keep logs for inspection but remove test files
rm -f $TEST_DIR/test_file.bin

echo "Test complete! Logs available in $TEST_DIR/" 