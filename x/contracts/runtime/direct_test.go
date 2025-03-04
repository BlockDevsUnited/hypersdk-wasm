// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"testing"

	"github.com/stretchr/testify/require"
)

// TestDirectContractCall tests directly calling a WASM function without compilation
func TestDirectContractCall(t *testing.T) {
	require := require.New(t)
	ctx := context.Background()

	// Create a test runtime
	rt := newTestRuntime(ctx)

	// Create a test contract bytes
	// This is a minimal WASM module that exports a function named wasm_call_with_two_params
	// The function returns a constant value of 3 (which should be serialized as [3, 0, 0, 0, 0, 0, 0, 0])
	contractBytes := []byte{
		// WASM binary header - magic constant ("\0asm") and version (1)
		0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,

		// Type section - define function signature
		0x01, 0x07, 0x01,
		0x60, // function type
		0x02, // 2 parameters 
		0x7F, 0x7F, // i32, i32 params
		0x01, // 1 return value
		0x7F, // i32 return

		// Function section - define one function with type 0
		0x03, 0x02, 0x01, 0x00,

		// Export section - export our function as "wasm_call_with_two_params"
		0x07, 0x19, 0x01,
		0x17, // length of export name
		// "wasm_call_with_two_params"
		0x77, 0x61, 0x73, 0x6d, 0x5f, 0x63, 0x61, 0x6c, 0x6c, 0x5f, 0x77, 0x69, 0x74, 0x68, 0x5f, 0x74, 0x77, 0x6f, 0x5f, 0x70, 0x61, 0x72, 0x61, 0x6d, 0x73,
		0x00, 0x00, // export function 0

		// Memory section - define memory
		0x05, 0x03, 0x01, 0x00, 0x01,

		// Code section - function bodies
		0x0a, 0x09, 0x01, // 1 function, 9 bytes of code
		0x07, // local decl count
		0x00, // 0 locals
		0x41, 0x03, // i32.const 3
		0x0b, // end
	}

	// Create a contract ID
	var contractID ContractID = make([]byte, 20)
	contractID[0] = 1

	// Get the contract manager from the runtime
	// Since the test runtime's StateManager is already TestStateManager, we use it directly
	err := rt.StateManager.(TestStateManager).CompileAndSetContract(contractID, "call_contract")
	if err != nil {
		// If compilation fails, set the bytes directly
		err = rt.StateManager.(TestStateManager).SetContractBytes(ctx, contractID, contractBytes)
		require.NoError(err)
	}

	// Create a contract
	contract, err := rt.newTestContract("call_contract")
	require.NoError(err)

	// Call the function
	expected, err := Serialize(int64(3))
	require.NoError(err)

	result, err := contract.Call("call_with_two_params", uint64(1), uint64(2))
	require.NoError(err)

	// Verify the result
	require.Equal(expected, result)
}
