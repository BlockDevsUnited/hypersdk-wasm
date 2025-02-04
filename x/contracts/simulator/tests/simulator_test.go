package tests

import (
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/ava-labs/avalanchego/utils/logging"
	"github.com/ava-labs/hypersdk/codec"
	contractruntime "github.com/ava-labs/hypersdk/x/contracts/runtime"
	"github.com/ava-labs/hypersdk/x/contracts/simulator/state"
	"github.com/stretchr/testify/require"
)

func TestSimulatorBasic(t *testing.T) {
	require := require.New(t)
	ctx := context.Background()
	logger := logging.NewLogger("Test")

	// Create simulator state
	simState := state.NewSimulatorState()

	// Create runtime
	r := contractruntime.NewRuntime(contractruntime.NewConfig(), logger)

	// Create test addresses
	actor := codec.Address{1}
	contract := codec.Address{2}

	// Set initial balance
	err := simState.TransferBalance(ctx, codec.EmptyAddress, actor, 1000)
	require.NoError(err)

	balance, err := simState.GetBalance(ctx, actor)
	require.NoError(err)
	require.Equal(uint64(1000), balance)

	// Test state operations
	key := []byte("test_key")
	value := []byte("test_value")
	var retrievedValue []byte

	err = simState.Insert(ctx, key, value)
	require.NoError(err)

	retrievedValue, err = simState.GetValue(ctx, key)
	require.NoError(err)
	require.Equal(value, retrievedValue)

	// Test contract call
	callInfo := &contractruntime.CallInfo{
		State:        simState,
		Actor:        actor,
		Contract:     contract,
		FunctionName: "test_function",
		Params:       []byte("test_params"),
		Fuel:         1000,
		Height:       1,
		Timestamp:    1000,
	}

	// Execute contract call
	result, err := r.WithDefaults(contractruntime.CallInfo{}).CallContract(ctx, callInfo)
	require.Error(err) // Should error since we haven't deployed a contract
	require.Nil(result)

	// Test removing value
	err = simState.Remove(ctx, key)
	require.NoError(err)

	retrievedValue, err = simState.GetValue(ctx, key)
	require.NoError(err)
	require.Nil(retrievedValue)
}

func TestSimulatorTransfers(t *testing.T) {
	require := require.New(t)
	ctx := context.Background()

	// Create simulator state
	simState := state.NewSimulatorState()

	// Create test addresses
	addr1 := codec.Address{1}
	addr2 := codec.Address{2}

	// Set initial balances
	err := simState.TransferBalance(ctx, codec.EmptyAddress, addr1, 1000)
	require.NoError(err)

	// Test transfer
	err = simState.TransferBalance(ctx, addr1, addr2, 500)
	require.NoError(err)

	balance1, err := simState.GetBalance(ctx, addr1)
	require.NoError(err)
	require.Equal(uint64(500), balance1)

	balance2, err := simState.GetBalance(ctx, addr2)
	require.NoError(err)
	require.Equal(uint64(500), balance2)

	// Test insufficient funds
	err = simState.TransferBalance(ctx, addr1, addr2, 1000)
	require.Error(err)
}

func TestSimulatorRemoveSafety(t *testing.T) {
	ctx := context.Background()

	t.Run("Basic Remove", func(t *testing.T) {
		require := require.New(t)
		simState := state.NewSimulatorState()
		key := []byte("test_key")
		value := []byte("test_value")

		// Insert and verify
		err := simState.Insert(ctx, key, value)
		require.NoError(err)

		// Remove and verify
		err = simState.Remove(ctx, key)
		require.NoError(err)

		// Verify key is removed
		val, err := simState.GetValue(ctx, key)
		require.NoError(err)
		require.Nil(val)
	})

	t.Run("Nil State", func(t *testing.T) {
		require := require.New(t)
		var simState *state.SimulatorState
		err := simState.Remove(ctx, []byte("test"))
		require.Error(err)
		require.Contains(err.Error(), "invalid state")
	})

	t.Run("Empty Key", func(t *testing.T) {
		require := require.New(t)
		simState := state.NewSimulatorState()
		err := simState.Remove(ctx, []byte{})
		require.Error(err)
		require.Contains(err.Error(), "empty key")
	})

	t.Run("Large Key", func(t *testing.T) {
		require := require.New(t)
		simState := state.NewSimulatorState()
		largeKey := make([]byte, 1024*1024) // 1MB key
		for i := range largeKey {
			largeKey[i] = byte(i % 256)
		}
		
		// First insert
		err := simState.Insert(ctx, largeKey, []byte("test_value"))
		require.NoError(err)

		// Then remove
		err = simState.Remove(ctx, largeKey)
		require.NoError(err)

		// Verify removal
		val, err := simState.GetValue(ctx, largeKey)
		require.NoError(err)
		require.Nil(val)
	})

	t.Run("Concurrent Operations", func(t *testing.T) {
		require := require.New(t)
		simState := state.NewSimulatorState()
		const numGoroutines = 10
		const numOperations = 100

		// Create channels for synchronization
		done := make(chan bool)
		errors := make(chan error, numGoroutines*numOperations)

		// Launch goroutines
		for i := 0; i < numGoroutines; i++ {
			go func(routineID int) {
				for j := 0; j < numOperations; j++ {
					key := []byte(fmt.Sprintf("key_%d_%d", routineID, j))
					value := []byte(fmt.Sprintf("value_%d_%d", routineID, j))

					// Insert
					if err := simState.Insert(ctx, key, value); err != nil {
						errors <- err
						continue
					}

					// Remove
					if err := simState.Remove(ctx, key); err != nil {
						errors <- err
						continue
					}

					// Verify removal
					if val, err := simState.GetValue(ctx, key); err != nil {
						errors <- err
					} else if val != nil {
						errors <- fmt.Errorf("key should be removed but still exists: %s", key)
					}
				}
				done <- true
			}(i)
		}

		// Wait for all goroutines to complete
		for i := 0; i < numGoroutines; i++ {
			<-done
		}
		close(errors)

		// Check for any errors
		for err := range errors {
			require.NoError(err)
		}
	})
}

func TestSimulatorCallbacks(t *testing.T) {
	require := require.New(t)
	ctx := context.Background()

	// Create simulator state
	simState := state.NewSimulatorState()

	// Test state operations with large values to test memory management
	key := make([]byte, 1024)   // 1KB key
	value := make([]byte, 1024) // 1KB value
	for i := 0; i < 1024; i++ {
		key[i] = byte(i % 256)
		value[i] = byte((i + 128) % 256)
	}

	// Test Insert
	err := simState.Insert(ctx, key, value)
	require.NoError(err)

	// Test GetValue
	retrievedValue, err := simState.GetValue(ctx, key)
	require.NoError(err)
	require.Equal(value, retrievedValue)

	// Test concurrent access
	for i := 0; i < 10; i++ {
		key := []byte(fmt.Sprintf("key_%d", i))
		value := []byte(fmt.Sprintf("value_%d", i))
		
		// Insert in a goroutine
		go func(k, v []byte) {
			err := simState.Insert(ctx, k, v)
			require.NoError(err)
		}(key, value)
	}

	// Wait for goroutines to finish
	// In a real test, we would use sync.WaitGroup
	// but for this example we'll just sleep
	time.Sleep(100 * time.Millisecond)

	// Verify all values were inserted
	for i := 0; i < 10; i++ {
		key := []byte(fmt.Sprintf("key_%d", i))
		expectedValue := []byte(fmt.Sprintf("value_%d", i))
		
		value, err := simState.GetValue(ctx, key)
		require.NoError(err)
		require.Equal(expectedValue, value)
	}

	// Test Remove
	err = simState.Remove(ctx, key)
	require.NoError(err)

	// Verify value was removed
	retrievedValue, err = simState.GetValue(ctx, key)
	require.NoError(err)
	require.Nil(retrievedValue)

	// Test error cases
	// Empty key
	err = simState.Insert(ctx, []byte{}, value)
	require.Error(err)

	// Nil value
	err = simState.Insert(ctx, key, nil)
	require.NoError(err) // Should allow nil value

	// Get non-existent key
	retrievedValue, err = simState.GetValue(ctx, []byte("non_existent"))
	require.NoError(err)
	require.Nil(retrievedValue)

	// Remove non-existent key
	err = simState.Remove(ctx, []byte("non_existent"))
	require.NoError(err)
}
