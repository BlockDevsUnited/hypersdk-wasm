// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"errors"
	"fmt"
	"sync"

	"github.com/ava-labs/avalanchego/database"
)

const (
	getCost    = 10000
	putCost    = 10000
	deleteCost = 10000

	putManyCost = 10000
)

type keyValueInput struct {
	Key   []byte
	Value []byte
}

// AsyncResult represents a pending asynchronous operation
type AsyncResult struct {
	// ID uniquely identifies this async operation
	ID string
	// Ready indicates if the result is available
	Ready bool
	// Value contains the result value (if Ready is true)
	Value []byte
	// Error contains any error that occurred (if Ready is true)
	Error error
	// CompletionChan is notified when the operation completes
	CompletionChan chan struct{}
}

// AsyncStateManager manages pending async operations
type AsyncStateManager struct {
	mu       sync.RWMutex
	results  map[string]*AsyncResult
	nextID   uint64
	registry *AsyncCallbackRegistry
}

// NewAsyncStateManager creates a new AsyncStateManager
func NewAsyncStateManager() *AsyncStateManager {
	return &AsyncStateManager{
		results:  make(map[string]*AsyncResult),
		registry: NewAsyncCallbackRegistry(),
	}
}

// GetResult retrieves a result by ID
func (a *AsyncStateManager) GetResult(id string) *AsyncResult {
	a.mu.RLock()
	defer a.mu.RUnlock()
	return a.results[id]
}

// RegisterResult registers a new async operation
func (a *AsyncStateManager) RegisterResult() *AsyncResult {
	a.mu.Lock()
	defer a.mu.Unlock()
	a.nextID++
	id := fmt.Sprintf("%d", a.nextID)
	result := &AsyncResult{
		ID:             id,
		Ready:          false,
		CompletionChan: make(chan struct{}),
	}
	a.results[id] = result
	return result
}

// CompleteResult marks an async operation as complete
func (a *AsyncStateManager) CompleteResult(id string, value []byte, err error) {
	a.mu.Lock()
	result, exists := a.results[id]
	if !exists {
		a.mu.Unlock()
		return
	}
	
	result.Ready = true
	result.Value = value
	result.Error = err
	a.mu.Unlock()
	
	// Notify waiters
	close(result.CompletionChan)
}

// AsyncRegistry tracks operations that need to resume after async completion
type AsyncRegistry struct {
	mu        sync.Mutex
	callbacks map[string]func([]byte, error)
}

// Register adds a callback to be executed when an async operation completes
func (r *AsyncRegistry) Register(id string, callback func([]byte, error)) {
	r.mu.Lock()
	defer r.mu.Unlock()
	if r.callbacks == nil {
		r.callbacks = make(map[string]func([]byte, error))
	}
	r.callbacks[id] = callback
}

// Execute executes the callback for the given operation ID
func (r *AsyncRegistry) Execute(id string, value []byte, err error) bool {
	r.mu.Lock()
	callback, exists := r.callbacks[id]
	if exists {
		delete(r.callbacks, id)
	}
	r.mu.Unlock()
	
	if exists {
		callback(value, err)
		return true
	}
	return false
}

func NewStateAccessModule() *ImportModule {
	asyncManager := NewAsyncStateManager()
	
	return &ImportModule{
		Name: "state",
		HostFunctions: map[string]HostFunction{
			"get": {FuelCost: getCost, Function: Function[[]byte, RawBytes](func(callInfo *CallInfo, input []byte) (RawBytes, error) {
				ctx, cancel := context.WithCancel(context.Background())
				defer cancel()
				val, err := callInfo.State.GetContractState(callInfo.Contract).GetValue(ctx, input)
				if err != nil {
					if errors.Is(err, database.ErrNotFound) {
						return nil, nil
					}
					return nil, err
				}
				return val, nil
			})},
			"get_async": {FuelCost: getCost, Function: Function[[]byte, string](func(callInfo *CallInfo, input []byte) (string, error) {
				// Register new async operation
				result := asyncManager.RegisterResult()
				
				// Start async operation
				go func() {
					ctx, cancel := context.WithCancel(context.Background())
					defer cancel()
					val, err := callInfo.State.GetContractState(callInfo.Contract).GetValue(ctx, input)
					if err != nil && errors.Is(err, database.ErrNotFound) {
						err = nil // Not found is not an error, just returns nil
					}
					asyncManager.CompleteResult(result.ID, val, err)
				}()
				
				// Return operation ID
				return result.ID, nil
			})},
			"check_async": {FuelCost: 1000, Function: Function[string, bool](func(callInfo *CallInfo, id string) (bool, error) {
				result := asyncManager.GetResult(id)
				if result == nil {
					return false, errors.New("invalid async operation ID")
				}
				return result.Ready, nil
			})},
			"get_async_result": {FuelCost: 1000, Function: Function[string, RawBytes](func(callInfo *CallInfo, id string) (RawBytes, error) {
				result := asyncManager.GetResult(id)
				if result == nil {
					return nil, errors.New("invalid async operation ID")
				}
				if !result.Ready {
					return nil, errors.New("async operation not complete")
				}
				return result.Value, result.Error
			})},
			"put": {FuelCost: putManyCost, Function: FunctionNoOutput[[]keyValueInput](func(callInfo *CallInfo, input []keyValueInput) error {
				ctx, cancel := context.WithCancel(context.Background())
				defer cancel()
				contractState := callInfo.State.GetContractState(callInfo.Contract)
				for _, entry := range input {
					if len(entry.Value) == 0 {
						if err := contractState.Remove(ctx, entry.Key); err != nil {
							return err
						}
					} else if err := contractState.Insert(ctx, entry.Key, entry.Value); err != nil {
						return err
					}
				}
				return nil
			})},
			"put_async": {FuelCost: putManyCost, Function: Function[[]keyValueInput, string](func(callInfo *CallInfo, input []keyValueInput) (string, error) {
				// Register new async operation
				result := asyncManager.RegisterResult()
				
				// Start async operation
				go func() {
					ctx, cancel := context.WithCancel(context.Background())
					defer cancel()
					contractState := callInfo.State.GetContractState(callInfo.Contract)
					var err error
					for _, entry := range input {
						if len(entry.Value) == 0 {
							if e := contractState.Remove(ctx, entry.Key); e != nil {
								err = e
								break
							}
						} else if e := contractState.Insert(ctx, entry.Key, entry.Value); e != nil {
							err = e
							break
						}
					}
					asyncManager.CompleteResult(result.ID, nil, err)
				}()
				
				// Return operation ID
				return result.ID, nil
			})},
		},
	}
}
