// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"encoding/binary"
	"fmt"
	"testing"

	"github.com/bytecodealliance/wasmtime-go/v25"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// TestLogger implements LoggerInterface for testing
type TestLogger struct {
	DebugLogs []string
	ErrorLogs []string
}

func (l *TestLogger) Debug(msg string) {
	l.DebugLogs = append(l.DebugLogs, msg)
}

func (l *TestLogger) Error(msg string) {
	l.ErrorLogs = append(l.ErrorLogs, msg)
}

func TestParameterFormatDetection(t *testing.T) {
	// Create Wasmtime environment
	engine := wasmtime.NewEngine()
	store := wasmtime.NewStore(engine)
	
	// Create a memory instance with 1 page (64KB)
	memoryType := wasmtime.NewMemoryType(1, true, 10)
	memory, err := wasmtime.NewMemory(store, memoryType)
	require.NoError(t, err)
	
	// Get memory data for direct manipulation in tests
	memData := memory.UnsafeData(store)
	
	// Create test logger
	logger := &TestLogger{}
	
	// Create memory reader
	reader := NewMemoryReader(memory, store, logger)

	t.Run("LengthPrefixedFormat", func(t *testing.T) {
		// Create test data with length prefix
		testData := []byte("This is test data for integration")
		
		// Create length prefix (4 bytes, little-endian)
		var lengthBytes [4]byte
		binary.LittleEndian.PutUint32(lengthBytes[:], uint32(len(testData)))
		
		// Copy length prefix to memory at offset 100
		copy(memData[100:], lengthBytes[:])
		
		// Copy data after length prefix
		copy(memData[104:], testData)
		
		// Read using our parameter reader
		result, err := reader.ReadParameters(100, 0)
		require.NoError(t, err)
		
		// Verify data was read correctly
		assert.Equal(t, testData, result)
		
		// Check that debug log contains detection message
		foundLog := false
		for _, log := range logger.DebugLogs {
			if fmt.Sprintf("Detected length-prefixed parameter format: length=%d", len(testData)) == log {
				foundLog = true
				break
			}
		}
		assert.True(t, foundLog, "Should log detection of length-prefixed format")
	})
	
	t.Run("DirectFormat", func(t *testing.T) {
		// Reset logs for this test
		logger.DebugLogs = nil
		
		// Create 32-byte test data (typical for contract IDs in direct format)
		testData := make([]byte, 32)
		for i := range testData {
			testData[i] = byte(i)
		}
		
		// Copy data directly to memory at offset 200
		copy(memData[200:], testData)
		
		// Read using our parameter reader with size hint
		result, err := reader.ReadParameters(200, 32)
		require.NoError(t, err)
		
		// Verify data was read correctly
		assert.Equal(t, testData, result)
		
		// Check debug log
		foundLog := false
		for _, log := range logger.DebugLogs {
			if fmt.Sprintf("Using direct parameter format with size=%d", 32) == log {
				foundLog = true
				break
			}
		}
		assert.True(t, foundLog, "Should log detection of direct format")
	})
	
	t.Run("AutoDetection_DirectFormat", func(t *testing.T) {
		// Reset logs for this test
		logger.DebugLogs = nil
		
		// Data that could be mistaken for a length prefix but is actually direct format
		// First 4 bytes are 0xFF, 0xFF, 0xFF, 0xFF which as uint32 would be an unreasonable length
		testData := make([]byte, DefaultDirectParamSize)
		for i := 0; i < 4; i++ {
			testData[i] = 0xFF // These bytes as uint32 would be an unreasonable length
		}
		for i := 4; i < len(testData); i++ {
			testData[i] = byte(i)
		}
		
		// Copy data directly to memory at offset 300
		copy(memData[300:], testData)
		
		// Read without specifying size (should detect direct format)
		result, err := reader.ReadParameters(300, 0)
		require.NoError(t, err)
		
		// Verify we got default size data
		assert.Equal(t, testData[:DefaultDirectParamSize], result)
		
		// Check debug log
		foundLog := false
		for _, log := range logger.DebugLogs {
			if fmt.Sprintf("Using direct parameter format with size=%d", DefaultDirectParamSize) == log {
				foundLog = true
				break
			}
		}
		assert.True(t, foundLog, "Should detect direct format when length prefix is invalid")
	})
	
	t.Run("MemoryBoundsCheck", func(t *testing.T) {
		// Try to read past memory bounds
		_, err := reader.ReadParameters(65530, 32)
		require.Error(t, err)
		assert.Contains(t, err.Error(), "memory access out of bounds")
	})
}

func TestMemoryWriter(t *testing.T) {
	// Create Wasmtime environment
	engine := wasmtime.NewEngine()
	store := wasmtime.NewStore(engine)
	
	// Create a memory instance with 1 page (64KB)
	memoryType := wasmtime.NewMemoryType(1, true, 10)
	memory, err := wasmtime.NewMemory(store, memoryType)
	require.NoError(t, err)
	
	// Get memory data for direct manipulation in tests
	memData := memory.UnsafeData(store)
	
	// Create test logger
	logger := &TestLogger{}
	
	// Create memory writer
	writer := NewMemoryWriter(memory, store, logger)
	reader := NewMemoryReader(memory, store, logger)

	t.Run("LengthPrefixedFormat", func(t *testing.T) {
		// Create test data
		testData := []byte("Integration test data")
		
		// Write using length-prefixed format
		err := writer.WriteParameters(400, testData, true)
		require.NoError(t, err)
		
		// Verify length prefix was written correctly
		lengthBytes := memData[400:404]
		length := binary.LittleEndian.Uint32(lengthBytes)
		assert.Equal(t, uint32(len(testData)), length)
		
		// Verify data was written correctly
		writtenData := memData[404:404+len(testData)]
		assert.Equal(t, testData, writtenData)
		
		// Read back using reader
		readData, err := reader.ReadParameters(400, 0)
		require.NoError(t, err)
		assert.Equal(t, testData, readData)
	})
	
	t.Run("DirectFormat", func(t *testing.T) {
		// Create test data
		testData := []byte("Direct format data")
		
		// Write using direct format
		err := writer.WriteParameters(500, testData, false)
		require.NoError(t, err)
		
		// Verify data was written correctly
		writtenData := memData[500:500+len(testData)]
		assert.Equal(t, testData, writtenData)
		
		// Read back using reader with explicit size
		readData, err := reader.ReadParameters(500, len(testData))
		require.NoError(t, err)
		assert.Equal(t, testData, readData)
	})
	
	t.Run("MemoryBoundsCheck", func(t *testing.T) {
		// Try to write past memory bounds
		err := writer.WriteParameters(65530, []byte("This should fail"), true)
		require.Error(t, err)
		assert.Contains(t, err.Error(), "memory access out of bounds")
	})
}
