package runtime

import (
	"context"
	"testing"

	"github.com/stretchr/testify/require"
)

// TestPatchedImportContractCallContractWithParams is a patched version that manually loads the WASM file
func TestPatchedImportContractCallContractWithParams(t *testing.T) {
	require := require.New(t)
	ctx := context.Background()

	// Create a new test runtime
	rt := newTestRuntime(ctx)
	
	// Create a new contract
	contract, err := rt.newTestContract("call_contract")
	require.NoError(err)

	expected, err := Serialize(int64(3))
	require.NoError(err)
	
	// Test the call_with_two_params function
	result, err := contract.Call(
		"call_with_two_params",
		uint64(1),
		uint64(2))
	require.NoError(err)
	
	require.Equal(expected, result)
}
