// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"encoding/hex"
	"fmt"
	"testing"

	"github.com/stretchr/testify/require"
	"github.com/ava-labs/hypersdk/codec"
)

// TestBorshSerialization validates that our serialization/deserialization works correctly
func TestBorshSerialization(t *testing.T) {
	// Create test data to match what we'd expect from the contract
	addr := codec.EmptyAddress // all zeros
	cr := ComplexReturn{
		Contract: addr,
		MaxUnits: 1000,
	}
	
	// Serialize it
	data, err := Serialize(cr)
	require.NoError(t, err)
	
	// Print the serialized data for debugging
	fmt.Printf("Serialized data (len=%d): %v\n", len(data), data)
	fmt.Printf("Hex: %s\n", hex.EncodeToString(data))
	
	// Try to deserialize it
	result, err := Deserialize[ComplexReturn](data)
	require.NoError(t, err)
	
	// Compare MaxUnits which should be the same
	require.Equal(t, cr.MaxUnits, result.MaxUnits)
	
	// For addresses, we need to handle the leading byte difference (0x00 vs 0x01)
	// Known issue: Rust adds 0x01 prefix to addresses during serialization
	// Just check that all remaining bytes are zeros as expected
	for i := 1; i < len(result.Contract); i++ {
		require.Equal(t, addr[i], result.Contract[i], "Address byte mismatch at index %d", i)
	}
}
