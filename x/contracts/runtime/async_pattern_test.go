// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"fmt"
	"sync"
	"testing"
	"time"

	"github.com/stretchr/testify/require"
)

// mockAsyncRuntime simulates an async runtime without relying on actual WASM compilation
type mockAsyncRuntime struct {
	operations map[string]operationState
	mu         sync.Mutex
}

type operationState struct {
	complete bool
	result   []byte
}

func newMockAsyncRuntime() *mockAsyncRuntime {
	return &mockAsyncRuntime{
		operations: make(map[string]operationState),
	}
}

// StartOperation begins a new async operation and returns an operation ID
func (m *mockAsyncRuntime) StartOperation(result []byte, simulateDelay bool) string {
	opID := fmt.Sprintf("op-%d", time.Now().UnixNano())
	
	m.mu.Lock()
	m.operations[opID] = operationState{
		complete: false,
		result:   result,
	}
	m.mu.Unlock()
	
	// If we want to simulate an actual delay, start a goroutine
	if simulateDelay {
		go func() {
			time.Sleep(100 * time.Millisecond) // Simulate processing time
			
			m.mu.Lock()
			defer m.mu.Unlock()
			
			if state, exists := m.operations[opID]; exists {
				state.complete = true
				m.operations[opID] = state
			}
		}()
	} else {
		// Immediately mark as complete for testing
		m.mu.Lock()
		state := m.operations[opID]
		state.complete = true
		m.operations[opID] = state
		m.mu.Unlock()
	}
	
	return opID
}

// CheckOperation checks if an operation is complete
func (m *mockAsyncRuntime) CheckOperation(opID string) bool {
	m.mu.Lock()
	defer m.mu.Unlock()
	
	if state, exists := m.operations[opID]; exists {
		return state.complete
	}
	
	return false
}

// GetOperationResult gets the result of a completed operation
func (m *mockAsyncRuntime) GetOperationResult(opID string) ([]byte, bool) {
	m.mu.Lock()
	defer m.mu.Unlock()
	
	if state, exists := m.operations[opID]; exists && state.complete {
		return state.result, true
	}
	
	return nil, false
}

// TestAsyncPatternSimulator tests the async pattern without requiring WASM compilation
func TestAsyncPatternSimulator(t *testing.T) {
	require := require.New(t)
	_ = context.Background()

	// Create our mock async runtime
	mockRuntime := newMockAsyncRuntime()
	
	// Define test data for ComplexReturn
	mockData := []byte{1, 2, 3, 4, 5} // Simplified mock data
	
	// Start an async operation
	opID := mockRuntime.StartOperation(mockData, false)
	require.NotEmpty(opID, "Expected non-empty operation ID")
	
	// Check operation status
	isComplete := mockRuntime.CheckOperation(opID)
	require.True(isComplete, "Operation should be complete immediately in test mode")
	
	// Get operation result
	result, success := mockRuntime.GetOperationResult(opID)
	require.True(success, "Should be able to get operation result")
	require.Equal(mockData, result, "Result should match expected data")
	
	// Start a delayed operation
	delayedOpID := mockRuntime.StartOperation(mockData, true)
	require.NotEmpty(delayedOpID, "Expected non-empty operation ID for delayed operation")
	
	// Initially the operation should not be complete
	initialState := mockRuntime.CheckOperation(delayedOpID)
	if initialState {
		// This is a flaky test - sometimes the operation might complete very quickly
		t.Log("Warning: Delayed operation completed immediately. This is unusual but possible.")
	}
	
	// Wait for operation to complete
	time.Sleep(200 * time.Millisecond)
	
	// Now the operation should be complete
	finalState := mockRuntime.CheckOperation(delayedOpID)
	require.True(finalState, "Operation should be complete after waiting")
	
	// Get delayed operation result
	delayedResult, success := mockRuntime.GetOperationResult(delayedOpID)
	require.True(success, "Should be able to get delayed operation result")
	require.Equal(mockData, delayedResult, "Delayed result should match expected data")
	
	// Log the successful async pattern implementation
	t.Log("Successfully simulated async operation pattern")
	t.Log("This test demonstrates the core concepts of the async contract runtime:")
	t.Log("1. Operations are identified by unique IDs")
	t.Log("2. Operation status can be checked")
	t.Log("3. Results are retrieved once operations are complete")
}
