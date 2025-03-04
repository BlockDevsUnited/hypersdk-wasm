// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"testing"

	"github.com/stretchr/testify/require"
)

// TestEnvModuleSetCallResult tests the set_call_result function in the env module directly
func TestEnvModuleSetCallResult(t *testing.T) {
	require := require.New(t)
	
	// Create a new env module which contains the set_call_result function
	envModule := NewEnvModule()
	
	// Check that the module has the set_call_result function
	setCallResultFunc, exists := envModule.HostFunctions["set_call_result"]
	require.True(exists, "set_call_result function should exist")
	
	// Verify that the set_call_result function has a reasonable fuel cost
	require.Greater(setCallResultFunc.FuelCost, uint64(0), "Fuel cost should be greater than 0")
	
	// The function should actually exist and not be nil
	require.NotNil(setCallResultFunc.Function, "Function should not be nil")
}
