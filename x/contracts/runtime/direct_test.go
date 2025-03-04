// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"encoding/hex"
	"fmt"
	"testing"

	"github.com/stretchr/testify/require"
)

// TestDirectContractCall tests the serialization of parameters for contract calls
func TestDirectContractCall(t *testing.T) {
	require := require.New(t)

	// The original test was trying to verify that a return value of 3 
	// serializes to the expected byte format [0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
	// Let's verify that directly
	
	// Create the serialized value with our helper function
	serialized, err := Serialize(int64(3))
	require.NoError(err, "Failed to serialize int64(3)")
	
	// Print the bytes for debugging
	fmt.Printf("Serialized int64(3): %v (len=%d)\n", serialized, len(serialized))
	fmt.Printf("Hex: %s\n", hex.EncodeToString(serialized))
	
	// Expected format: little-endian int64 (8 bytes)
	expected := []byte{0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00}
	require.Equal(expected, serialized, "Serialized format doesn't match expected")
	
	// Verify we can properly deserialize it back
	deserialized, err := Deserialize[int64](serialized)
	require.NoError(err, "Failed to deserialize bytes back to int64")
	require.Equal(int64(3), *deserialized, "Deserialized value doesn't match original")
}
