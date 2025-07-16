#!/bin/bash

# Simple test script to verify interactive command execution
echo "Testing interactive command execution fix..."

# Test with a simple interactive command - this should work now
echo "Testing with: echo 'Hello World'"
cargo run ask "execute the command: echo 'Hello World'"

echo ""
echo "Test completed."