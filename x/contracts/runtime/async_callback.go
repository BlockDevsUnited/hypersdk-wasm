// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"encoding/binary"
	"sync"
	"time"

	"github.com/bytecodealliance/wasmtime-go/v25"
)

// AsyncCallback represents a reference to a WebAssembly function
// that should be called when an asynchronous operation completes.
// This structure maintains all context needed for the callback.
type AsyncCallback struct {
	// WebAssembly function pointer to call when operation completes
	FunctionPtr uint32

	// Parameters to pass to the function
	Parameters []byte

	// Parameter format (0=length-prefixed, 1=direct)
	ParamFormat uint8

	// Maximum time the operation can run
	Timeout time.Duration

	// Time when the operation was created
	CreatedAt time.Time

	// WebAssembly memory instance
	Memory *wasmtime.Memory

	// Call context
	CallInfo *CallInfo

	// Is this an operation running in a TEE environment
	IsTEEOperation bool

	// Optional operation-specific context
	Context interface{}
}

// AsyncCallbackRegistry tracks and manages WebAssembly callback functions
// for asynchronous operations.
type AsyncCallbackRegistry struct {
	mu        sync.Mutex
	callbacks map[string]*AsyncCallback
}

// NewAsyncCallbackRegistry creates a new registry for async callbacks
func NewAsyncCallbackRegistry() *AsyncCallbackRegistry {
	return &AsyncCallbackRegistry{
		callbacks: make(map[string]*AsyncCallback),
	}
}

// RegisterCallback registers a new callback for an operation ID
func (r *AsyncCallbackRegistry) RegisterCallback(id string, callback *AsyncCallback) {
	r.mu.Lock()
	defer r.mu.Unlock()
	r.callbacks[id] = callback
}

// GetCallback retrieves a callback by operation ID
func (r *AsyncCallbackRegistry) GetCallback(id string) *AsyncCallback {
	r.mu.Lock()
	defer r.mu.Unlock()
	return r.callbacks[id]
}

// RemoveCallback removes a callback by operation ID
func (r *AsyncCallbackRegistry) RemoveCallback(id string) {
	r.mu.Lock()
	defer r.mu.Unlock()
	delete(r.callbacks, id)
}

// FormatLengthPrefixedParameter formats a byte slice as a length-prefixed parameter
func FormatLengthPrefixedParameter(data []byte) []byte {
	result := make([]byte, len(data)+4)
	binary.LittleEndian.PutUint32(result, uint32(len(data)))
	copy(result[4:], data)
	return result
}

// ParseParameterData extracts data from either length-prefixed or direct parameter format
func ParseParameterData(data []byte, expectedDirectSize int) ([]byte, bool) {
	// Must have at least 4 bytes to check for length prefix
	if len(data) >= 4 {
		// Read potential length as little-endian u32
		potentialLen := binary.LittleEndian.Uint32(data[:4])
		
		// Check if the potential length is reasonable and fits in the buffer
		if potentialLen > 0 && potentialLen < 1048576 && potentialLen+4 <= uint32(len(data)) {
			// It's a length-prefixed parameter
			return data[4:4+potentialLen], true
		}
	}
	
	// If length prefix detection fails or we expect a direct size, treat as direct format
	if expectedDirectSize > 0 && expectedDirectSize <= len(data) {
		return data[:expectedDirectSize], false
	}
	
	// Default case - return all data as direct format
	return data, false
}
