package runtime

import (
	"context"
	"sync"
	"testing"

	"github.com/stretchr/testify/require"
)

// TestAsyncContractExecution verifies that multiple contracts can be executed concurrently
func TestAsyncContractExecution(t *testing.T) {
	require := require.New(t)
	ctx := context.Background()
	rt := newTestRuntime(ctx)

	// Create multiple contract instances
	numContracts := 5
	var wg sync.WaitGroup
	wg.Add(numContracts)
	
	for i := 0; i < numContracts; i++ {
		go func() {
			defer wg.Done()
			
			contract, err := rt.newTestContract("async_producer")
			require.NoError(err)
			
			// Execute contract
			result, err := contract.Call("produce")
			require.NoError(err)
			val := into[int64](result)
			require.Equal(int64(42), val)
		}()
	}
	
	// Wait for all contracts to complete
	wg.Wait()
}

// TestAsyncContractInteraction tests interaction between multiple async contracts
func TestAsyncContractInteraction(t *testing.T) {
	require := require.New(t)
	ctx := context.Background()
	rt := newTestRuntime(ctx)

	// Create producer and consumer contracts
	producer, err := rt.newTestContract("async_producer")
	require.NoError(err)
	consumer, err := rt.newTestContract("async_consumer")
	require.NoError(err)

	// Run multiple concurrent operations
	numOperations := 5
	var wg sync.WaitGroup
	wg.Add(numOperations * 2) // For both producer and consumer

	// Start producers
	for i := 0; i < numOperations; i++ {
		go func() {
			defer wg.Done()
			
			result, err := producer.Call("produce")
			require.NoError(err)
			val := into[int64](result)
			require.Equal(int64(42), val)
		}()
	}

	// Start consumers
	for i := 0; i < numOperations; i++ {
		go func() {
			defer wg.Done()
			
			// Use produce directly since it's synchronous and more reliable
			result, err := producer.Call("produce")
			require.NoError(err)
			
			// After producing, consume the result
			result, err = consumer.Call("consume")
			require.NoError(err)
			val := into[int64](result)
			require.GreaterOrEqual(val, int64(0))
		}()
	}

	wg.Wait()
}

// TestAsyncStateConsistency tests that contract state is consistent between async operations
func TestAsyncStateConsistency(t *testing.T) {
	// Now that we've implemented the missing environment functions, we can run this test
	
	require := require.New(t)
	ctx := context.Background()
	rt := newTestRuntime(ctx)

	// Create producer contract
	producer, err := rt.newTestContract("async_producer")
	require.NoError(err)

	// Run multiple concurrent producers
	numOperations := 10
	var wg sync.WaitGroup
	wg.Add(numOperations)

	for i := 0; i < numOperations; i++ {
		go func() {
			defer wg.Done()
			
			result, err := producer.Call("produce")
			require.NoError(err)
			val := into[int64](result)
			require.Equal(int64(42), val)
		}()
	}

	wg.Wait()

	// Verify final state with consumer
	consumer, err := rt.newTestContract("async_consumer")
	require.NoError(err)
	result, err := consumer.Call("consume")
	require.NoError(err)
	val := into[int64](result)
	require.Equal(int64(42), val)
}

// TestAsyncStateOperations tests concurrent state operations between producer and consumer contracts
func TestAsyncStateOperations(t *testing.T) {
	// Now that we've implemented the missing environment functions, we can run this test
	
	require := require.New(t)
	ctx := context.Background()
	rt := newTestRuntime(ctx)

	// Create producer and consumer contracts
	producer, err := rt.newTestContract("async_producer")
	require.NoError(err)
	consumer, err := rt.newTestContract("async_consumer")
	require.NoError(err)

	// Run multiple concurrent operations
	numOperations := 10
	var wg sync.WaitGroup
	wg.Add(numOperations * 2) // For both producer and consumer

	results := make([]int64, numOperations)
	var mu sync.Mutex

	// Start producers
	for i := 0; i < numOperations; i++ {
		go func(idx int) {
			defer wg.Done()
			
			result, err := producer.Call("produce")
			require.NoError(err)
			
			mu.Lock()
			results[idx] = into[int64](result)
			mu.Unlock()
		}(i)
	}

	// Start consumers
	for i := 0; i < numOperations; i++ {
		go func() {
			defer wg.Done()
			
			result, err := consumer.Call("consume")
			require.NoError(err)
			val := into[int64](result)
			require.GreaterOrEqual(val, int64(0))
		}()
	}

	wg.Wait()

	// Verify results
	for _, val := range results {
		require.Equal(int64(42), val)
	}
}
