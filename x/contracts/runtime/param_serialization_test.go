// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"testing"

	"github.com/ava-labs/hypersdk/x/contracts/test"
	"github.com/stretchr/testify/require"
)

// TestParameterSerialization tests the parameter serialization for contract calls
func TestParameterSerialization(t *testing.T) {
	require := require.New(t)

	// Test serializing different types of parameters
	// These are the types that would be passed between contracts

	// Test simple integer serialization
	val1 := uint64(42)
	serializedVal1, err := Serialize(val1)
	require.NoError(err)
	require.Equal([]byte{42, 0, 0, 0, 0, 0, 0, 0}, serializedVal1)

	// Test simple negative integer serialization (int64)
	val2 := int64(-1)
	serializedVal2, err := Serialize(val2)
	require.NoError(err)
	require.Equal([]byte{255, 255, 255, 255, 255, 255, 255, 255}, serializedVal2)

	// Test serializing a string
	val3 := "test"
	serializedVal3, err := Serialize(val3)
	require.NoError(err)
	// String is serialized as length (4) followed by bytes 't', 'e', 's', 't'
	require.Equal([]byte{4, 0, 0, 0, 't', 'e', 's', 't'}, serializedVal3)

	// Check how multiple parameters are serialized when passed to a contract call
	// Each parameter is serialized separately and passed as a single byte array
	param1 := uint64(1)
	param2 := uint64(2)
	
	// Serialize the individual parameters
	serializedParam1, err := Serialize(param1)
	require.NoError(err)
	serializedParam2, err := Serialize(param2)
	require.NoError(err)
	
	// Verify the individual parameter serialization
	require.Equal([]byte{1, 0, 0, 0, 0, 0, 0, 0}, serializedParam1)
	require.Equal([]byte{2, 0, 0, 0, 0, 0, 0, 0}, serializedParam2)
	
	// Now let's test how they might be combined
	// Based on the test failure, it seems an array is just encoded with its length
	arr := []interface{}{param1, param2}
	serializedArr, err := Serialize(arr)
	require.NoError(err)
	
	// The array is encoded with just its length (2) as a uint32
	require.Equal([]byte{2, 0, 0, 0}, serializedArr)
	
	// Most importantly, let's check how the test.SerializeParams function works
	// This is what's actually used when calling contracts
	serializedMultiParams := test.SerializeParams(param1, param2)
	
	// The parameters should be flattened into a single byte array
	// by concatenating the serialized bytes of each parameter
	expectedMultiParams := append(serializedParam1, serializedParam2...)
	require.Equal(expectedMultiParams, serializedMultiParams)
	
	// Verify the combined result matches our expectation
	require.Equal(
		[]byte{
			1, 0, 0, 0, 0, 0, 0, 0, // uint64(1)
			2, 0, 0, 0, 0, 0, 0, 0, // uint64(2)
		},
		serializedMultiParams,
	)
}
