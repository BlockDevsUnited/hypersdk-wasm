package state

import (
	"context"
	"testing"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/stretchr/testify/require"
)

func TestContractState(t *testing.T) {
	ctx := context.Background()
	sim := NewSimulatorState()
	require.NotNil(t, sim)

	// Create two different contract addresses
	contract1 := codec.Address{1, 2, 3}
	contract2 := codec.Address{4, 5, 6}

	// Get contract states
	state1 := sim.GetContractState(contract1)
	state2 := sim.GetContractState(contract2)
	require.NotNil(t, state1)
	require.NotNil(t, state2)

	// Test that contract states are isolated
	key := []byte("test_key")
	value1 := []byte("value1")
	value2 := []byte("value2")

	// Insert values for both contracts
	err := state1.Insert(ctx, key, value1)
	require.NoError(t, err)
	err = state2.Insert(ctx, key, value2)
	require.NoError(t, err)

	// Verify each contract can read its own value
	got1, err := state1.GetValue(ctx, key)
	require.NoError(t, err)
	require.Equal(t, value1, got1)

	got2, err := state2.GetValue(ctx, key)
	require.NoError(t, err)
	require.Equal(t, value2, got2)

	// Remove value from contract1 and verify it doesn't affect contract2
	err = state1.Remove(ctx, key)
	require.NoError(t, err)

	got1, err = state1.GetValue(ctx, key)
	require.NoError(t, err)
	require.Nil(t, got1)

	got2, err = state2.GetValue(ctx, key)
	require.NoError(t, err)
	require.Equal(t, value2, got2)
}

func TestContractStateErrors(t *testing.T) {
	ctx := context.Background()
	sim := NewSimulatorState()
	require.NotNil(t, sim)

	contract := codec.Address{1, 2, 3}
	state := sim.GetContractState(contract)
	require.NotNil(t, state)

	// Test empty key
	err := state.Insert(ctx, []byte{}, []byte("value"))
	require.Error(t, err)

	_, err = state.GetValue(ctx, []byte{})
	require.Error(t, err)

	err = state.Remove(ctx, []byte{})
	require.Error(t, err)
}
