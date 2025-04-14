// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"fmt"
	"testing"

	"github.com/ava-labs/avalanchego/utils/logging"
	"github.com/stretchr/testify/require"
	"github.com/bytecodealliance/wasmtime-go/v25"
	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/state"
)

// MemoryState is a simple in-memory implementation of both
// state.Mutable and ContractManager for testing
type MemoryState struct {
	balance    map[string]uint64
	contracts  map[string]ContractID
	contractsBytes map[string][]byte
	data       map[string][]byte
}

func NewMemoryState() *MemoryState {
	return &MemoryState{
		balance:   make(map[string]uint64),
		contracts: make(map[string]ContractID),
		contractsBytes: make(map[string][]byte),
		data:      make(map[string][]byte),
	}
}

// Implement state.Mutable interface
func (m *MemoryState) GetValue(ctx context.Context, key []byte) ([]byte, error) {
	value, exists := m.data[string(key)]
	if !exists {
		return nil, nil // Not found, return nil without error
	}
	return value, nil
}

func (m *MemoryState) Insert(ctx context.Context, key []byte, value []byte) error {
	m.data[string(key)] = value
	return nil
}

func (m *MemoryState) Remove(ctx context.Context, key []byte) error {
	delete(m.data, string(key))
	return nil
}

func (m *MemoryState) Commit() error {
	return nil // No-op for in-memory store
}

// StateManager interface implementation
type memoryStateManager struct {
	ctx   context.Context
	state *MemoryState
	asyncRegistry *AsyncRegistry
}

func NewStateManager(ctx context.Context, state *MemoryState, asyncRegistry *AsyncRegistry) StateManager {
	if state == nil {
		state = NewMemoryState()
	}
	return &memoryStateManager{
		ctx:   ctx,
		state: state,
		asyncRegistry: asyncRegistry,
	}
}

// StateManager implementation for testing
func (m *memoryStateManager) GetBalance(ctx context.Context, address codec.Address) (uint64, error) {
	return m.state.balance[string(address[:])], nil
}

func (m *memoryStateManager) TransferBalance(ctx context.Context, from codec.Address, to codec.Address, amount uint64) error {
	fromKey := string(from[:])
	toKey := string(to[:])
	
	if m.state.balance[fromKey] < amount {
		return fmt.Errorf("insufficient balance")
	}
	
	m.state.balance[fromKey] -= amount
	m.state.balance[toKey] += amount
	
	return nil
}

func (m *memoryStateManager) GetContractState(address codec.Address) state.Mutable {
	return m.state
}

func (m *memoryStateManager) GetAccountContract(ctx context.Context, account codec.Address) (ContractID, error) {
	contractID, exists := m.state.contracts[string(account[:])]
	if !exists {
		return nil, fmt.Errorf("contract not found")
	}
	return contractID, nil
}

func (m *memoryStateManager) GetContractBytes(ctx context.Context, contractID ContractID) ([]byte, error) {
	bytes, exists := m.state.contractsBytes[string(contractID)]
	if !exists {
		return nil, fmt.Errorf("contract bytes not found")
	}
	return bytes, nil
}

func (m *memoryStateManager) NewAccountWithContract(ctx context.Context, contractID ContractID, accountCreationData []byte) (codec.Address, error) {
	// Simplified for test - create a fake address based on contractID
	address := codec.Address{}
	copy(address[:], contractID)
	m.state.contracts[string(address[:])] = contractID
	return address, nil
}

func (m *memoryStateManager) SetAccountContract(ctx context.Context, account codec.Address, contractID ContractID) error {
	m.state.contracts[string(account[:])] = contractID
	return nil
}

func (m *memoryStateManager) SetContractBytes(ctx context.Context, contractID ContractID, contractBytes []byte) error {
	m.state.contractsBytes[string(contractID)] = contractBytes
	return nil
}

func TestCallValue(t *testing.T) {
	// Create a test state manager
	stateManager := NewStateManager(context.Background(), &MemoryState{}, nil)

	// Set our test value that we want to retrieve from the host function
	testValue := uint64(123456)

	// Setup wasmtime engine & store
	engine := wasmtime.NewEngine()
	_ = wasmtime.NewStore(engine)  // Unused but kept for reference

	// Create our runtime with a call info that has our test value
	rt := NewRuntime(NewConfig(), logging.NoLog{})
	callInfo := &CallInfo{                     
		Value: testValue,
		State: stateManager,
		Fuel:  1000000,
	}
	
	// Store the callInfo in the runtime
	rt.callInfo.Store(callInfo)
	
	// Call the host function directly to verify it works
	result := rt.envGetCallValue()
	
	// Verify we got the expected value back
	require.Equal(t, testValue, result, "Call value should match what was set in CallInfo")
}

func TestCallValueDirect(t *testing.T) {
	// This is a direct test of the get_call_value host function without requiring WASM compilation
	
	// Create a test state manager
	stateManager := NewStateManager(context.Background(), &MemoryState{}, nil)
	
	// Set our test value
	testValue := uint64(123456)
	
	// Create a CallInfo with our test value
	callInfo := &CallInfo{
		Value: testValue,
		State: stateManager,
		Fuel:  1000000,
	}
	
	// Create a runtime with our configuration
	rt := NewRuntime(NewConfig(), logging.NoLog{})
	
	// Set the call info directly in the runtime
	rt.callInfo.Store(callInfo)
	
	// Call the get_call_value host function directly
	result := rt.envGetCallValue()
	
	// Verify we got the expected value back
	require.Equal(t, testValue, result, "Call value should match what was set in CallInfo")
}
