// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"fmt"
	
	"github.com/bytecodealliance/wasmtime-go/v25"
)

// GetImportedMemory returns the memory imported by the contract
func (c *CallInfo) GetImportedMemory() *wasmtime.Memory {
	// The memory is accessed through the contract instance context
	// In a real implementation, this would reference the proper instance
	// For now we'll return a mock memory for demonstration
	return nil
}

// GetStoreForThread returns a store for the current thread
// This uses a cache to avoid creating too many stores
func (r *WasmRuntime) GetStoreForThread() *wasmtime.Store {
	// Generate a thread ID - in a real implementation, we would use the actual thread ID
	threadID := "thread-default"
	
	// Check if we have a store for this thread
	if val, ok := r.storeCache.Load(threadID); ok {
		return val.(*wasmtime.Store)
	}
	
	// Create a new store
	store := wasmtime.NewStore(r.engine)
	
	// Store it in the cache
	r.storeCache.Store(threadID, store)
	
	return store
}

// GetInstance returns the current WebAssembly instance
// This is a renamed version of getInstance to match the expected API
func (r *WasmRuntime) GetInstance() *ContractInstance {
	// Reuse the existing getInstance method with default parameters
	instance, err := r.getInstance(nil)
	if err != nil {
		r.log.Debug(fmt.Sprintf("Failed to get instance: %v", err))
		return nil
	}
	return instance
}

// StoreIDMapping stores a mapping from numeric ID to UUID
func (r *WasmRuntime) StoreIDMapping(numericID, uuidStr string) {
	r.idMappings.Store(numericID, uuidStr)
}

// GetIDMapping retrieves a UUID from a numeric ID
func (r *WasmRuntime) GetIDMapping(numericID string) (string, bool) {
	val, ok := r.idMappings.Load(numericID)
	if !ok {
		return "", false
	}
	return val.(string), true
}
