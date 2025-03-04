// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"encoding/binary"
	"fmt"
	"sync"
	"testing"
	"time"

	"github.com/stretchr/testify/require"
)

// AsyncContext represents a context for tracking async operations
type AsyncContext struct {
	operations      map[string]*AsyncOperation
	mu              sync.RWMutex
	contractAddress []byte
}

// AsyncOperation represents an asynchronous operation
type AsyncOperation struct {
	ID        string
	Status    AsyncStatus
	Result    []byte
	CreatedAt time.Time
	Contract  []byte
}

// AsyncStatus represents the status of an async operation
type AsyncStatus int

const (
	StatusPending AsyncStatus = iota
	StatusComplete
	StatusFailed
)

// NewAsyncContext creates a new async context
func NewAsyncContext(contractAddress []byte) *AsyncContext {
	return &AsyncContext{
		operations:      make(map[string]*AsyncOperation),
		contractAddress: contractAddress,
	}
}

// StartOperation begins a new async operation
func (c *AsyncContext) StartOperation(result []byte, delay time.Duration) string {
	opID := fmt.Sprintf("async-op-%d", time.Now().UnixNano())
	
	operation := &AsyncOperation{
		ID:        opID,
		Status:    StatusPending,
		Result:    result,
		CreatedAt: time.Now(),
		Contract:  c.contractAddress,
	}
	
	c.mu.Lock()
	c.operations[opID] = operation
	c.mu.Unlock()
	
	// Simulate async execution with a goroutine
	go func() {
		time.Sleep(delay)
		
		c.mu.Lock()
		defer c.mu.Unlock()
		
		if op, exists := c.operations[opID]; exists {
			op.Status = StatusComplete
		}
	}()
	
	return opID
}

// CheckOperation checks if an operation is complete
func (c *AsyncContext) CheckOperation(opID string) bool {
	c.mu.RLock()
	defer c.mu.RUnlock()
	
	if op, exists := c.operations[opID]; exists {
		return op.Status == StatusComplete
	}
	
	return false
}

// GetOperationResult gets the result of a completed operation
func (c *AsyncContext) GetOperationResult(opID string) ([]byte, bool) {
	c.mu.RLock()
	defer c.mu.RUnlock()
	
	if op, exists := c.operations[opID]; exists && op.Status == StatusComplete {
		return op.Result, true
	}
	
	return nil, false
}

// MockContract represents a contract that can perform async operations
type MockContract struct {
	context *AsyncContext
}

// NewMockContract creates a new mock contract
func NewMockContract(contractAddress []byte) *MockContract {
	return &MockContract{
		context: NewAsyncContext(contractAddress),
	}
}

// StoreValueAsync simulates storing a value asynchronously
func (c *MockContract) StoreValueAsync(value int64) string {
	// Convert value to bytes
	valueBytes := make([]byte, 8)
	binary.LittleEndian.PutUint64(valueBytes, uint64(value))
	
	// Start async operation with a short delay
	return c.context.StartOperation(valueBytes, 50*time.Millisecond)
}

// GetValueAsync gets a value asynchronously
func (c *MockContract) GetValueAsync(opID string) (int64, bool) {
	// Check if operation is complete
	if !c.context.CheckOperation(opID) {
		return 0, false
	}
	
	// Get operation result
	result, success := c.context.GetOperationResult(opID)
	if !success || len(result) < 8 {
		return 0, false
	}
	
	// Convert bytes to value
	value := int64(binary.LittleEndian.Uint64(result))
	return value, true
}

// TestAsyncContractModel tests the async contract model
func TestAsyncContractModel(t *testing.T) {
	require := require.New(t)
	_ = context.Background()
	
	// Create a mock contract
	contractAddress := []byte{1, 2, 3, 4}
	contract := NewMockContract(contractAddress)
	
	// Store a value asynchronously
	testValue := int64(42)
	opID := contract.StoreValueAsync(testValue)
	require.NotEmpty(opID, "Expected non-empty operation ID")
	
	// Initially the operation is not complete
	initialValue, initialSuccess := contract.GetValueAsync(opID)
	
	// This assertion might be flaky if the operation completes very quickly
	if initialSuccess {
		require.Equal(testValue, initialValue, "Initial value should match test value if completed")
		t.Log("Note: Async operation completed immediately - unusual but possible")
	}
	
	// Wait for operation to complete
	time.Sleep(100 * time.Millisecond)
	
	// Now the operation should be complete
	value, success := contract.GetValueAsync(opID)
	require.True(success, "Operation should be complete after waiting")
	require.Equal(testValue, value, "Retrieved value should match stored value")
	
	// Log success
	t.Log("Successfully demonstrated async contract pattern")
	t.Log("This model shows how to:")
	t.Log("1. Generate operation IDs")
	t.Log("2. Store operations in a context")
	t.Log("3. Check operation status")
	t.Log("4. Retrieve operation results")
	t.Log("5. Implement async methods in contracts")
}
