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
	
	// Validate
	require.Equal(t, cr, *result)
}
