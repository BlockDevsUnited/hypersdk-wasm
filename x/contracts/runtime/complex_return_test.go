// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"bytes"
	"encoding/hex"
	"fmt"
	"strings"
	"testing"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/stretchr/testify/require"
)

// This test will help diagnose the issue with ComplexReturn deserialization

// WasmlAddress is a 32-byte address used in Wasmlanche contracts
type WasmlAddress [32]byte

// ComplexReturnRust matches the Rust struct
type ComplexReturnRust struct {
	Contract WasmlAddress
	MaxUnits uint64
}

// ComplexReturnGo matches the Go struct from runtime_test.go
type ComplexReturnGo struct {
	Contract codec.Address // 33 bytes
	MaxUnits uint64
}

// Adds more debugging functions for tracing exact deserialization issues
func hexDump(data []byte, label string) {
	fmt.Printf("--- %s (%d bytes) ---\n", label, len(data))
	for i := 0; i < len(data); i += 16 {
		end := i + 16
		if end > len(data) {
			end = len(data)
		}
		
		// Print hex values
		fmt.Printf("%04x  ", i)
		for j := i; j < end; j++ {
			fmt.Printf("%02x ", data[j])
			if j-i == 7 {
				fmt.Print(" ")
			}
		}
		
		// Fill with spaces if the last line is incomplete
		if end-i < 16 {
			spaces := (16 - (end - i)) * 3
			if end-i <= 8 {
				spaces++
			}
			fmt.Print(strings.Repeat(" ", spaces))
		}
		
		// Print ASCII representation
		fmt.Print(" |")
		for j := i; j < end; j++ {
			if data[j] >= 32 && data[j] <= 126 {
				fmt.Printf("%c", data[j])
			} else {
				fmt.Print(".")
			}
		}
		fmt.Println("|")
	}
	fmt.Println()
}

func TestComplexReturnSerialization(t *testing.T) {
	r := require.New(t)
	
	// Create test addresses
	// When we create contract in Go, the first byte is the type ID (1)
	goAddr := codec.Address{}
	goAddr[0] = 1 // type byte for contract
	copy(goAddr[1:], bytes.Repeat([]byte{0xaa}, 32))
	
	// Create a ComplexReturn with known values
	testStruct := ComplexReturn{
		Contract: goAddr,
		MaxUnits: 1000,
	}
	
	// Serialize the test struct
	serialized, err := Serialize(testStruct)
	r.NoError(err)
	hexDump(serialized, "Serialized")
	
	// Deserialize the bytes back into the struct
	deserialized, err := Deserialize[ComplexReturn](serialized)
	r.NoError(err)
	r.Equal(testStruct.MaxUnits, deserialized.MaxUnits)
}

func TestComplexReturnSerialization2(t *testing.T) {
	r := require.New(t)
	
	// Create a test address with sequential bytes for easy debugging
	var rustAddr WasmlAddress
	for i := 0; i < 32; i++ {
		rustAddr[i] = byte(i)
	}
	
	// Create a Go address with extra byte at the beginning (type byte)
	var goAddr codec.Address
	goAddr[0] = 1 // Type byte
	for i := 0; i < 32; i++ {
		goAddr[i+1] = byte(i)
	}
	
	// Create the Rust-style struct
	rustStruct := ComplexReturnRust{
		Contract: rustAddr,
		MaxUnits: 1234,
	}
	
	// Create the Go-style struct
	goStruct := ComplexReturnGo{
		Contract: goAddr,
		MaxUnits: 1234,
	}
	
	// Serialize both structs
	rustBytes, err := Serialize(rustStruct)
	r.NoError(err)
	hexDump(rustBytes, "Rust Serialized")
	
	goBytes, err := Serialize(goStruct)
	r.NoError(err)
	hexDump(goBytes, "Go Serialized")
	
	// Print both byte arrays
	fmt.Printf("Rust bytes (%d): %s\n", len(rustBytes), hex.EncodeToString(rustBytes))
	fmt.Printf("Go bytes (%d): %s\n", len(goBytes), hex.EncodeToString(goBytes))
	
	// Try to deserialize the Rust bytes into the Go struct
	goResult, err := Deserialize[ComplexReturnGo](rustBytes)
	if err != nil {
		fmt.Printf("Error deserializing Rust bytes to Go struct: %v\n", err)
	} else {
		fmt.Printf("Successfully deserialized Rust bytes to Go struct: %+v\n", *goResult)
	}
	
	// Try to deserialize the Go bytes into the Rust struct
	rustResult, err := Deserialize[ComplexReturnRust](goBytes)
	if err != nil {
		fmt.Printf("Error deserializing Go bytes to Rust struct: %v\n", err)
	} else {
		fmt.Printf("Successfully deserialized Go bytes to Rust struct: %+v\n", *rustResult)
	}
	
	// Create an adapter function that can convert between the two formats
	rustFromGo := func(data []byte) ([]byte, error) {
		// Parse as the Go struct
		result, err := Deserialize[ComplexReturnGo](data)
		if err != nil {
			return nil, fmt.Errorf("error deserializing Go struct: %w", err)
		}
		
		// Convert to the Rust struct
		var rustAddr WasmlAddress
		copy(rustAddr[:], result.Contract[1:]) // Skip the type byte
		
		rustStruct := ComplexReturnRust{
			Contract: rustAddr,
			MaxUnits: result.MaxUnits,
		}
		
		// Re-serialize as the Rust struct
		return Serialize(rustStruct)
	}
	
	goFromRust := func(data []byte) ([]byte, error) {
		// Parse as the Rust struct
		result, err := Deserialize[ComplexReturnRust](data)
		if err != nil {
			return nil, fmt.Errorf("error deserializing Rust struct: %w", err)
		}
		
		// Convert to the Go struct
		var goAddr codec.Address
		goAddr[0] = 1 // Type byte
		copy(goAddr[1:], result.Contract[:])
		
		goStruct := ComplexReturnGo{
			Contract: goAddr,
			MaxUnits: result.MaxUnits,
		}
		
		// Re-serialize as the Go struct
		return Serialize(goStruct)
	}
	
	// Test the adapter functions
	adaptedRustBytes, err := rustFromGo(goBytes)
	r.NoError(err)
	hexDump(adaptedRustBytes, "Adapted Rust Serialized")
	
	adaptedGoBytes, err := goFromRust(rustBytes)
	r.NoError(err)
	hexDump(adaptedGoBytes, "Adapted Go Serialized")
	
	fmt.Printf("Adapted Rust bytes (%d): %s\n", len(adaptedRustBytes), hex.EncodeToString(adaptedRustBytes))
	fmt.Printf("Adapted Go bytes (%d): %s\n", len(adaptedGoBytes), hex.EncodeToString(adaptedGoBytes))
	
	// Test that we can now deserialize the adapted bytes correctly
	finalRustResult, err := Deserialize[ComplexReturnRust](adaptedRustBytes)
	r.NoError(err)
	r.Equal(rustStruct.MaxUnits, finalRustResult.MaxUnits)
	
	finalGoResult, err := Deserialize[ComplexReturnGo](adaptedGoBytes)
	r.NoError(err)
	r.Equal(goStruct.MaxUnits, finalGoResult.MaxUnits)
}
