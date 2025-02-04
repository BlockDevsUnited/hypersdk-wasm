package state

/*
#cgo CFLAGS: -I../common
#cgo LDFLAGS: -L${SRCDIR}/../common -lcallbacks
#include "callbacks.h"
#include "types.h"
#include <stdlib.h>
#include <string.h>

// Forward declarations of callback functions
BytesWithError get_value_callback(void* data, Bytes key);
char* insert_value_callback(void* data, Bytes key, Bytes value);
char* remove_value_callback(void* data, Bytes key);

// Forward declarations of bridge functions
BytesWithError bridge_get_callback(GetStateCallback callback, void* stateObj, Bytes key);
char* bridge_insert_callback(InsertStateCallback insertFuncPtr, void *dbPtr, Bytes key, Bytes value);
char* bridge_remove_callback(RemoveStateCallback removeFuncPtr, void *dbPtr, Bytes key);
Mutable new_mutable(void* stateObj, GetStateCallback get_cb, InsertStateCallback insert_cb, RemoveStateCallback remove_cb);
*/
import "C"

import (
	"context"
	"crypto/rand"
	"encoding/binary"
	"fmt"
	goruntime "runtime"
	"sync"
	"unsafe"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/state"
	"github.com/ava-labs/hypersdk/x/contracts/runtime"
)

var (
	stateMap = make(map[uint64]*SimulatorState)
	stateMu  sync.RWMutex
)

// SimulatorState implements runtime.StateManager
type SimulatorState struct {
	id      uint64
	data    map[string][]byte
	mu      sync.RWMutex
	mutable C.Mutable
}

// ContractState implements state.Mutable for a specific contract
type ContractState struct {
	*SimulatorState
	address codec.Address
}

// GetValue returns the value associated with the key for a specific contract.
func (c *ContractState) GetValue(ctx context.Context, key []byte) ([]byte, error) {
	if len(key) == 0 {
		return nil, fmt.Errorf("empty key")
	}

	// Prefix the key with the contract address to isolate contract state
	prefixedKey := append(c.address[:], key...)
	return c.SimulatorState.GetValue(ctx, prefixedKey)
}

// Insert inserts a key-value pair into the state for a specific contract.
func (c *ContractState) Insert(ctx context.Context, key []byte, value []byte) error {
	if len(key) == 0 {
		return fmt.Errorf("empty key")
	}

	// Prefix the key with the contract address to isolate contract state
	prefixedKey := append(c.address[:], key...)
	return c.SimulatorState.Insert(ctx, prefixedKey, value)
}

// Remove removes a key-value pair from the state for a specific contract.
func (c *ContractState) Remove(ctx context.Context, key []byte) error {
	if len(key) == 0 {
		return fmt.Errorf("empty key")
	}

	// Prefix the key with the contract address to isolate contract state
	prefixedKey := append(c.address[:], key...)
	return c.SimulatorState.Remove(ctx, prefixedKey)
}

// GetContractState returns the state of the contract at the given address.
func (s *SimulatorState) GetContractState(address codec.Address) state.Mutable {
	return &ContractState{
		SimulatorState: s,
		address:       address,
	}
}

//export get_value_callback
func get_value_callback(data unsafe.Pointer, key C.Bytes) C.BytesWithError {
	if data == nil {
		errStr := C.CString("nil state object")
		defer C.free(unsafe.Pointer(errStr))
		return C.BytesWithError{
			bytes: C.Bytes{
				data:   nil,
				length: 0,
			},
			error: errStr,
		}
	}

	// Convert pointer to state ID
	stateID := *(*uint64)(data)

	// Get state from map
	stateMu.RLock()
	s, ok := stateMap[stateID]
	stateMu.RUnlock()
	if !ok {
		errStr := C.CString("invalid state object")
		defer C.free(unsafe.Pointer(errStr))
		return C.BytesWithError{
			bytes: C.Bytes{
				data:   nil,
				length: 0,
			},
			error: errStr,
		}
	}

	// Convert C bytes to Go bytes
	var keyBytes []byte
	if key.data != nil && key.length > 0 {
		keyBytes = C.GoBytes(unsafe.Pointer(key.data), C.int(key.length))
	}

	// Call GetValue
	value, err := s.GetValue(context.Background(), keyBytes)
	if err != nil {
		errStr := C.CString(err.Error())
		defer C.free(unsafe.Pointer(errStr))
		return C.BytesWithError{
			bytes: C.Bytes{
				data:   nil,
				length: 0,
			},
			error: errStr,
		}
	}

	// Handle nil or empty value case
	if value == nil || len(value) == 0 {
		return C.BytesWithError{
			bytes: C.Bytes{
				data:   nil,
				length: 0,
			},
			error: nil,
		}
	}

	// Allocate memory in C and copy the data
	cData := C.malloc(C.size_t(len(value)))
	if cData == nil {
		errStr := C.CString("failed to allocate memory")
		defer C.free(unsafe.Pointer(errStr))
		return C.BytesWithError{
			bytes: C.Bytes{
				data:   nil,
				length: 0,
			},
			error: errStr,
		}
	}
	C.memcpy(cData, unsafe.Pointer(&value[0]), C.size_t(len(value)))

	return C.BytesWithError{
		bytes: C.Bytes{
			data:   (*C.uchar)(cData),
			length: C.size_t(len(value)),
		},
		error: nil,
	}
}

//export insert_value_callback
func insert_value_callback(data unsafe.Pointer, key C.Bytes, value C.Bytes) *C.char {
	if data == nil {
		return C.CString("nil state object")
	}

	// Convert pointer to state ID
	stateID := *(*uint64)(data)

	// Get state from map
	stateMu.RLock()
	s, ok := stateMap[stateID]
	stateMu.RUnlock()
	if !ok {
		return C.CString("invalid state object")
	}

	// Convert C bytes to Go bytes
	keyBytes := C.GoBytes(unsafe.Pointer(key.data), C.int(key.length))
	valueBytes := C.GoBytes(unsafe.Pointer(value.data), C.int(value.length))

	// Call Insert
	if err := s.Insert(context.Background(), keyBytes, valueBytes); err != nil {
		return C.CString(err.Error())
	}

	return nil
}

//export remove_value_callback
func remove_value_callback(data unsafe.Pointer, key C.Bytes) *C.char {
	if data == nil {
		return C.CString("nil state object")
	}

	// Convert pointer to state ID
	stateID := *(*uint64)(data)

	// Get state from map
	stateMu.RLock()
	s, ok := stateMap[stateID]
	stateMu.RUnlock()
	if !ok {
		return C.CString("invalid state object")
	}

	// Convert C bytes to Go bytes
	keyBytes := C.GoBytes(unsafe.Pointer(key.data), C.int(key.length))

	// Call Remove
	if err := s.Remove(context.Background(), keyBytes); err != nil {
		return C.CString(err.Error())
	}

	return nil
}

// generateStateID generates a random state ID
func generateStateID() (uint64, error) {
	var id uint64
	err := binary.Read(rand.Reader, binary.BigEndian, &id)
	if err != nil {
		return 0, fmt.Errorf("failed to generate state ID: %w", err)
	}
	return id, nil
}

// WrapSimulatorState creates a new SimulatorState from an unsafe pointer
func WrapSimulatorState(ptr unsafe.Pointer) *SimulatorState {
	// Convert pointer to state ID
	stateID := *(*uint64)(ptr)

	// Get state from map
	stateMu.RLock()
	state, ok := stateMap[stateID]
	stateMu.RUnlock()

	if !ok {
		// If state doesn't exist, create a new one with the given ID
		state = &SimulatorState{
			id:   stateID,
			data: make(map[string][]byte),
		}

		// Store state in global map
		stateMu.Lock()
		stateMap[stateID] = state
		stateMu.Unlock()

		// Set finalizer to clean up state when it's garbage collected
		goruntime.SetFinalizer(state, func(s *SimulatorState) {
			stateMu.Lock()
			delete(stateMap, s.id)
			stateMu.Unlock()
		})

		// Create a new Mutable struct with callbacks
		state.mutable = C.new_mutable(
			unsafe.Pointer(&state.id),
			(C.GetStateCallback)(C.get_value_callback),
			(C.InsertStateCallback)(C.insert_value_callback),
			(C.RemoveStateCallback)(C.remove_value_callback),
		)
	}

	return state
}

// NewSimulatorState creates a new simulator state
func NewSimulatorState() *SimulatorState {
	// Generate a unique state ID
	id, err := generateStateID()
	if err != nil {
		panic(err)
	}

	// Create a new simulator state
	state := &SimulatorState{
		id:   id,
		data: make(map[string][]byte),
	}

	// Store state in global map
	stateMu.Lock()
	stateMap[id] = state
	stateMu.Unlock()

	// Set finalizer to clean up state when it's garbage collected
	goruntime.SetFinalizer(state, func(s *SimulatorState) {
		stateMu.Lock()
		delete(stateMap, s.id)
		stateMu.Unlock()
	})

	// Create a new Mutable struct with callbacks
	state.mutable = C.new_mutable(
		unsafe.Pointer(&state.id),
		(C.GetStateCallback)(C.get_value_callback),
		(C.InsertStateCallback)(C.insert_value_callback),
		(C.RemoveStateCallback)(C.remove_value_callback),
	)

	return state
}

// balanceKey returns the key used to store the balance for an address
func balanceKey(addr codec.Address) string {
	return fmt.Sprintf("balance:%s", addr)
}

// GetValue returns the value associated with the key.
func (s *SimulatorState) GetValue(ctx context.Context, key []byte) ([]byte, error) {
	if s == nil {
		return nil, fmt.Errorf("invalid state")
	}

	if len(key) == 0 {
		return nil, fmt.Errorf("empty key")
	}

	s.mu.RLock()
	value, ok := s.data[string(key)]
	s.mu.RUnlock()

	if !ok {
		return nil, nil
	}

	return value, nil
}

// Insert inserts a key-value pair into the state.
func (s *SimulatorState) Insert(ctx context.Context, key []byte, value []byte) error {
	if s == nil {
		return fmt.Errorf("invalid state")
	}

	if len(key) == 0 {
		return fmt.Errorf("empty key")
	}

	s.mu.Lock()
	s.data[string(key)] = value
	s.mu.Unlock()

	return nil
}

// Remove removes a key-value pair from the state.
func (s *SimulatorState) Remove(ctx context.Context, key []byte) error {
	if s == nil {
		return fmt.Errorf("invalid state")
	}

	if len(key) == 0 {
		return fmt.Errorf("empty key")
	}

	s.mu.Lock()
	delete(s.data, string(key))
	s.mu.Unlock()

	return nil
}

// GetAccountContract returns the contract ID associated with the given account.
func (s *SimulatorState) GetAccountContract(ctx context.Context, account codec.Address) (runtime.ContractID, error) {
	if s == nil {
		return runtime.ContractID{}, fmt.Errorf("nil state")
	}

	value, err := s.GetValue(ctx, account[:])
	if err != nil {
		return runtime.ContractID{}, err
	}

	if len(value) != 32 {
		return runtime.ContractID{}, fmt.Errorf("invalid contract ID length")
	}

	var contractID runtime.ContractID
	copy(contractID[:], value)
	return contractID, nil
}

// GetContractBytes returns the compiled WASM bytes of the contract with the given ID.
func (s *SimulatorState) GetContractBytes(ctx context.Context, contractID runtime.ContractID) ([]byte, error) {
	if s == nil {
		return nil, fmt.Errorf("nil state")
	}

	return s.GetValue(ctx, contractID[:])
}

// SetContractBytes stores the compiled WASM bytes of a contract.
func (s *SimulatorState) SetContractBytes(ctx context.Context, contractID runtime.ContractID, code []byte) error {
	if s == nil {
		return fmt.Errorf("nil state")
	}

	return s.Insert(ctx, contractID[:], code)
}

// NewAccountWithContract creates a new account that represents a specific instance of a contract.
func (s *SimulatorState) NewAccountWithContract(ctx context.Context, contractID runtime.ContractID, accountCreationData []byte) (codec.Address, error) {
	if s == nil {
		return codec.Address{}, fmt.Errorf("nil state")
	}

	// Generate a new random address
	var address codec.Address
	_, err := rand.Read(address[:])
	if err != nil {
		return codec.Address{}, fmt.Errorf("failed to generate random address: %w", err)
	}

	// Set the contract ID for the new account
	err = s.SetAccountContract(ctx, address, contractID)
	if err != nil {
		return codec.Address{}, fmt.Errorf("failed to set account contract: %w", err)
	}

	return address, nil
}

// SetAccountContract associates a contract ID with an account.
func (s *SimulatorState) SetAccountContract(ctx context.Context, account codec.Address, contractID runtime.ContractID) error {
	if s == nil {
		return fmt.Errorf("nil state")
	}

	return s.Insert(ctx, account[:], contractID[:])
}

// GetBalance returns the balance of the given address.
func (s *SimulatorState) GetBalance(ctx context.Context, addr codec.Address) (uint64, error) {
	if s == nil {
		return 0, fmt.Errorf("nil state")
	}

	s.mu.RLock()
	value, ok := s.data[balanceKey(addr)]
	s.mu.RUnlock()

	if !ok {
		return 0, nil
	}

	return binary.BigEndian.Uint64(value), nil
}

// TransferBalance transfers balance from one address to another
func (s *SimulatorState) TransferBalance(ctx context.Context, from, to codec.Address, amount uint64) error {
	if s == nil {
		return fmt.Errorf("nil state")
	}

	// Special case: if from is empty address, we're minting new tokens
	if from != codec.EmptyAddress {
		fromBalance, err := s.GetBalance(ctx, from)
		if err != nil {
			return err
		}
		if fromBalance < amount {
			return fmt.Errorf("insufficient balance")
		}

		// Update from balance
		newFromBalance := fromBalance - amount
		fromBalanceBytes := make([]byte, 8)
		binary.BigEndian.PutUint64(fromBalanceBytes, newFromBalance)
		s.mu.Lock()
		s.data[balanceKey(from)] = fromBalanceBytes
		s.mu.Unlock()
	}

	// Update to balance
	toBalance, err := s.GetBalance(ctx, to)
	if err != nil {
		return err
	}
	newToBalance := toBalance + amount
	toBalanceBytes := make([]byte, 8)
	binary.BigEndian.PutUint64(toBalanceBytes, newToBalance)
	s.mu.Lock()
	s.data[balanceKey(to)] = toBalanceBytes
	s.mu.Unlock()

	return nil
}

// Ensure SimulatorState implements state.Mutable and runtime.StateManager
var (
	_ state.Mutable = &SimulatorState{}
	_ runtime.StateManager = &SimulatorState{}
)
