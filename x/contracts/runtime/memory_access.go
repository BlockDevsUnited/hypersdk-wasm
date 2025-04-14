// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"encoding/binary"
	"fmt"

	"github.com/bytecodealliance/wasmtime-go/v25"
	"go.uber.org/zap"
)

// LoggerInterface defines the minimum logging interface required by memory utilities
// This is a simplified version that our memory utilities use internally
type LoggerInterface interface {
	Debug(msg string)
	Error(msg string)
}

// LoggerAdapter adapts WasmRuntime's logger to the simpler LoggerInterface
type LoggerAdapter struct {
	originalLogger interface{
		Debug(string, ...zap.Field)
		Error(string, ...zap.Field)
	}
}

// NewLoggerAdapter creates a new adapter for the WasmRuntime's logger
func NewLoggerAdapter(logger interface{
	Debug(string, ...zap.Field)
	Error(string, ...zap.Field)
}) *LoggerAdapter {
	return &LoggerAdapter{originalLogger: logger}
}

// Debug implements the LoggerInterface Debug method
func (a *LoggerAdapter) Debug(msg string) {
	a.originalLogger.Debug(msg)
}

// Error implements the LoggerInterface Error method
func (a *LoggerAdapter) Error(msg string) {
	a.originalLogger.Error(msg)
}

const (
	// MaxParameterSize is the maximum allowed parameter size (1MB)
	MaxParameterSize = 1024 * 1024

	// DirectParamMaxSize is the size threshold for determining if a parameter is
	// using the direct format vs the length-prefixed format
	DirectParamMaxSize = 1024

	// DefaultDirectParamSize is the default size for commonly used direct parameter format
	// (like contract IDs which are 32 bytes)
	DefaultDirectParamSize = 32
)

// MemoryReader provides common operations for reading from WebAssembly memory
// with support for both length-prefixed and direct parameter formats
type MemoryReader struct {
	memory *wasmtime.Memory
	store  *wasmtime.Store
	log    LoggerInterface
}

// NewMemoryReader creates a new memory reader for the given memory and store
func NewMemoryReader(memory *wasmtime.Memory, store *wasmtime.Store, log LoggerInterface) *MemoryReader {
	return &MemoryReader{
		memory: memory,
		store:  store,
		log:    log,
	}
}

// ReadParameters reads parameters from WebAssembly memory, handling both format types:
// 1. Length-prefixed: first 4 bytes are a little-endian uint32 length, followed by data
// 2. Direct: data is passed directly without length prefix
//
// This function automatically detects the appropriate format and returns the data.
func (r *MemoryReader) ReadParameters(ptr uint32, expectedDirectSize int) ([]byte, error) {
	// First try to read as length-prefixed
	var lengthBytes [4]byte

	// Read the potential length prefix (first 4 bytes)
	if err := r.readBytes(ptr, lengthBytes[:]); err != nil {
		return nil, fmt.Errorf("failed to read potential length prefix: %w", err)
	}

	// Decode the potential length
	length := binary.LittleEndian.Uint32(lengthBytes[:])

	// Determine if this is likely a length-prefixed format by checking if:
	// 1. Length is reasonable (0 < length <= MaxParameterSize)
	// 2. Length is not likely to be mistaken for data (avoid false positives)
	if length > 0 && length <= MaxParameterSize {
		r.log.Debug(fmt.Sprintf("Detected length-prefixed parameter format: length=%d", length))
		
		// This appears to be a length-prefixed format
		data := make([]byte, length)
		if err := r.readBytes(ptr+4, data); err != nil {
			return nil, fmt.Errorf("failed to read length-prefixed data: %w", err)
		}
		
		return data, nil
	}

	// If we're here, it doesn't appear to be a valid length-prefixed format
	// Try direct format with the expected size
	size := expectedDirectSize
	if size <= 0 {
		// If no expected size provided, use default
		size = DefaultDirectParamSize
	}

	r.log.Debug(fmt.Sprintf("Using direct parameter format with size=%d", size))
	
	// Read as direct format
	data := make([]byte, size)
	if err := r.readBytes(ptr, data); err != nil {
		return nil, fmt.Errorf("failed to read direct format data: %w", err)
	}
	
	return data, nil
}

// readBytes reads the specified number of bytes from WebAssembly memory
func (r *MemoryReader) readBytes(offset uint32, data []byte) error {
	// Ensure the memory is large enough
	memSize := uint64(r.memory.DataSize(r.store))
	if uint64(offset)+uint64(len(data)) > memSize {
		return fmt.Errorf("memory access out of bounds: offset=%d, length=%d, memory size=%d", 
			offset, len(data), memSize)
	}
	
	// Get direct access to memory
	linearMem := r.memory.UnsafeData(r.store)
	
	// Copy the data
	copy(data, linearMem[offset:offset+uint32(len(data))])
	
	return nil
}

// MemoryWriter provides common operations for writing to WebAssembly memory
type MemoryWriter struct {
	memory *wasmtime.Memory
	store  *wasmtime.Store
	log    LoggerInterface
}

// NewMemoryWriter creates a new memory writer for the given memory and store
func NewMemoryWriter(memory *wasmtime.Memory, store *wasmtime.Store, log LoggerInterface) *MemoryWriter {
	return &MemoryWriter{
		memory: memory,
		store:  store,
		log:    log,
	}
}

// WriteParameters writes parameters to WebAssembly memory
// If useLengthPrefix is true, it will use the length-prefixed format
// Otherwise, it will write the data directly
func (w *MemoryWriter) WriteParameters(ptr uint32, data []byte, useLengthPrefix bool) error {
	if useLengthPrefix {
		// First write the length as a 4-byte little-endian uint32
		var lengthBytes [4]byte
		binary.LittleEndian.PutUint32(lengthBytes[:], uint32(len(data)))
		
		if err := w.writeBytes(ptr, lengthBytes[:]); err != nil {
			return fmt.Errorf("failed to write length prefix: %w", err)
		}
		
		// Then write the actual data after the length
		if err := w.writeBytes(ptr+4, data); err != nil {
			return fmt.Errorf("failed to write data: %w", err)
		}
		
		return nil
	}
	
	// Direct format - just write the data
	return w.writeBytes(ptr, data)
}

// writeBytes writes the specified bytes to WebAssembly memory
func (w *MemoryWriter) writeBytes(offset uint32, data []byte) error {
	// Ensure the memory is large enough
	memSize := uint64(w.memory.DataSize(w.store))
	if uint64(offset)+uint64(len(data)) > memSize {
		return fmt.Errorf("memory access out of bounds: offset=%d, length=%d, memory size=%d", 
			offset, len(data), memSize)
	}
	
	// Get direct access to memory
	linearMem := w.memory.UnsafeData(w.store)
	
	// Copy the data
	copy(linearMem[offset:], data)
	
	return nil
}
