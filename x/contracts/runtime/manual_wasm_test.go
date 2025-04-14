// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"os"
	"path/filepath"
	"testing"

	"github.com/stretchr/testify/require"
)

// TestContractCallBasic works by creating a precompiled WASM module
// that can be loaded directly without relying on Cargo compilation
func TestContractCallBasic(t *testing.T) {
	require := require.New(t)
	
	// Skip the test if we're in a environment without proper Cargo setup
	// This allows the test to be run in environments with Cargo but skip in CI
	if os.Getenv("SKIP_CARGO_TESTS") == "true" {
		t.Skip("Skipping test that requires Cargo compilation")
	}
	
	ctx := context.Background()
	
	// Create a minimal working WASM module that returns a constant value
	// This is a minimal WASM module that exports a function that returns an int64(3)
	wasmBytes := []byte{
		// WASM binary header - magic constant ("\0asm") and version (1)
		0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00,
		
		// Type section - define function signature
		0x01, 0x07, 0x01,
		0x60, // function type
		0x00, // 0 parameters
		0x01, // 1 return value
		0x7F, // i32 return
		
		// Function section - define one function with type 0
		0x03, 0x02, 0x01, 0x00,
		
		// Export section - export our function as "test_function"
		0x07, 0x0D, 0x01,
		0x0C, // length of export name
		// "test_function"
		0x74, 0x65, 0x73, 0x74, 0x5f, 0x66, 0x75, 0x6e, 0x63, 0x74, 0x69, 0x6f, 0x6e,
		0x00, 0x00, // export function 0
		
		// Code section - function bodies
		0x0a, 0x09, 0x01, // 1 function, 9 bytes of code
		0x07, // local decl count
		0x00, // 0 locals
		0x41, 0x03, // i32.const 3
		0x0b, // end
	}
	
	// Create a temporary directory to store our WASM file
	tempDir, err := os.MkdirTemp("", "wasm-test")
	require.NoError(err)
	defer os.RemoveAll(tempDir) // clean up
	
	// Create contract directories structure
	contractDir := filepath.Join(tempDir, "simple")
	targetDir := filepath.Join(contractDir, "target", "wasm32-unknown-unknown", "release")
	require.NoError(os.MkdirAll(targetDir, 0755))
	
	// Write our WASM module to the file
	wasmPath := filepath.Join(targetDir, "simple.wasm")
	require.NoError(os.WriteFile(wasmPath, wasmBytes, 0644))
	
	// Now create a runtime
	rt := newTestRuntime(ctx)
	
	// Create a contract manually using our file
	contractID := ContractID([]byte{1})
	
	// Set the contract bytes
	err = rt.StateManager.(TestStateManager).SetContractBytes(ctx, contractID, wasmBytes)
	require.NoError(err)
	
	// Create a wrapper for our contract
	contract := &testContract{
		ID:      contractID,
		Runtime: rt,
	}
	
	// Call our function
	result, err := contract.Call("test_function")
	require.NoError(err)
	
	// Verify the result is what we expect (int64(3) serialized)
	expected, err := Serialize(int64(3))
	require.NoError(err)
	require.Equal(expected, result)
}
