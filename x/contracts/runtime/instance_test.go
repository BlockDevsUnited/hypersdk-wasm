// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"fmt"
	"strings"
	"testing"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/x/contracts/test"
	"github.com/bytecodealliance/wasmtime-go/v25"
	"github.com/stretchr/testify/require"
)

func TestEphemeralInstanceExecute(t *testing.T) {
	require := require.New(t)
	ctx := context.Background()

	// Create a test contract
	contractID := ids.GenerateTestID()
	contractAccount := codec.CreateAddress(0, contractID)
	stringedID := string(contractID[:])

	// Setup contract manager and state
	contractManager := NewContractStateManager(test.NewTestDB(), []byte{})
	err := contractManager.SetAccountContract(ctx, contractAccount, ContractID(stringedID))
	require.NoError(err)
	testStateManager := &TestStateManager{
		ContractManager: contractManager,
		Balances:        make(map[codec.Address]uint64),
	}

	// Compile and set the test contract
	err = testStateManager.CompileAndSetContract(ContractID(stringedID), "call_contract")
	require.NoError(err)

	// Create runtime with imports and defaults
	r := NewRuntime(
		NewConfig(),
		nil, // No logger needed for test
	).WithDefaults(CallInfo{
		State:    testStateManager,
		Contract: contractAccount,
		Fuel:     1000000,
	})

	// Create test actor
	actor := codec.CreateAddress(1, ids.GenerateTestID())
	actionID := ids.GenerateTestID()

	// Create call info
	callInfo := &CallInfo{
		FunctionName: "simple_call",
		ActionID:     actionID,
	}

	// Execute the contract
	result, err := r.WithActor(actor).CallContract(ctx, callInfo)
	require.NoError(err)
	require.Equal([]byte{0, 0, 0, 0, 0, 0, 0, 0}, result) // simple_call returns 0 as i64
}

func validateModuleMemory(engine *wasmtime.Engine, module *wasmtime.Module, maxPages uint32) error {
	// Check number of exports
	exports := module.Exports()
	if len(exports) > 1000 {
		return fmt.Errorf("number of exports %d exceeds maximum allowed 1000", len(exports))
	}

	// Check module's memory type before instantiation
	for _, imp := range module.Imports() {
		if imp.Type().MemoryType() != nil {
			memType := imp.Type().MemoryType()
			min := memType.Minimum()
			if min > uint64(maxPages) {
				return fmt.Errorf("imported memory minimum pages %d exceeds maximum allowed %d", min, maxPages)
			}
		}
	}

	for _, exp := range exports {
		if exp.Type().MemoryType() != nil {
			memType := exp.Type().MemoryType()
			min := memType.Minimum()
			if min > uint64(maxPages) {
				return fmt.Errorf("exported memory minimum pages %d exceeds maximum allowed %d", min, maxPages)
			}
			ok, maxPages64 := memType.Maximum()
			if ok && maxPages64 > uint64(maxPages) {
				return fmt.Errorf("exported memory maximum pages %d exceeds maximum allowed %d", maxPages64, maxPages)
			}
		}
	}

	// Create store and linker
	store := wasmtime.NewStore(engine)
	linker := wasmtime.NewLinker(store.Engine)
	err := linker.DefineWasi()
	if err != nil {
		return fmt.Errorf("failed to define WASI: %w", err)
	}

	// Create memory with our limits
	memoryType := wasmtime.NewMemoryType(
		uint32(1), // min pages
		true,      // shared
		maxPages,  // max pages
	)

	memory, err := wasmtime.NewMemory(store, memoryType)
	if err != nil {
		return fmt.Errorf("failed to create memory: %w", err)
	}

	err = linker.Define(store, "", "memory", memory)
	if err != nil {
		return fmt.Errorf("failed to define memory: %w", err)
	}

	// Try to instantiate with our memory limits
	_, err = linker.Instantiate(store, module)
	if err != nil {
		errStr := err.Error()
		if strings.Contains(errStr, "memory") || strings.Contains(errStr, "pages") {
			return fmt.Errorf("memory limits exceeded: %w", err)
		}
		return fmt.Errorf("module instantiation failed: %w", err)
	}

	return nil
}

func TestResourceLimits(t *testing.T) {
	tests := []struct {
		name    string
		wat     string
		wantErr bool
	}{
		{
			name: "valid_memory_limits",
			wat: `(module
				(memory 1 16)
				(export "memory" (memory 0))
			)`,
			wantErr: false,
		},
		{
			name: "exceeds_max_initial_memory",
			wat: `(module
				(memory 17 32)
				(export "memory" (memory 0))
			)`,
			wantErr: true,
		},
		{
			name: "exceeds_max_memory_growth",
			wat: `(module
				(memory 1 32)
				(export "memory" (memory 0))
			)`,
			wantErr: true,
		},
		{
			name: "too_many_exports",
			wat: func() string {
				exports := make([]string, 0, 1001)
				for i := 0; i < 1001; i++ {
					exports = append(exports, fmt.Sprintf(`(export "func_%d" (func $dummy))`, i))
				}
				return fmt.Sprintf(`
					(module
						(func $dummy)
						%s
					)`, strings.Join(exports, "\n"))
			}(),
			wantErr: true,
		},
	}

	engine := wasmtime.NewEngine()

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Convert WAT to WASM
			wasm, err := wasmtime.Wat2Wasm(tt.wat)
			if err != nil {
				if !tt.wantErr {
					t.Fatalf("Wat2Wasm() error = %v", err)
				}
				return
			}

			// Create module
			module, err := wasmtime.NewModule(engine, wasm)
			if err != nil {
				if !tt.wantErr {
					t.Fatalf("NewModule() error = %v", err)
				}
				return
			}

			err = validateModuleMemory(engine, module, 16)
			if (err != nil) != tt.wantErr {
				t.Errorf("validateModuleMemory() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestMemoryGrowth(t *testing.T) {
	// Test contract that tries to grow memory
	wat := `(module
		(memory 1 1)  ;; Initial 1 page, max 1 page (can't grow)
		(func (export "grow") (result i32)
			i32.const 1    ;; Number of pages to grow
			memory.grow    ;; Try to grow memory
		)
	)`

	// Convert WAT to WASM
	engine := wasmtime.NewEngine()
	wasm, err := wasmtime.Wat2Wasm(wat)
	if err != nil {
		t.Fatalf("Wat2Wasm() error = %v", err)
	}

	// Create and instantiate module
	module, err := wasmtime.NewModule(engine, wasm)
	if err != nil {
		t.Fatalf("NewModule() error = %v", err)
	}

	store := wasmtime.NewStore(engine)
	linker := wasmtime.NewLinker(engine)
	err = linker.DefineWasi()
	require.NoError(t, err)

	instance, err := linker.Instantiate(store, module)
	if err != nil {
		t.Fatalf("Instantiate() error = %v", err)
	}

	// Get grow function
	grow := instance.GetExport(store, "grow")
	if grow == nil {
		t.Fatal("grow function not found")
	}

	// Try to grow memory
	growFunc := grow.Func()
	if growFunc == nil {
		t.Fatal("grow is not a function")
	}

	val, err := growFunc.Call(store)
	if err != nil {
		t.Fatalf("grow() error = %v", err)
	}

	// Should fail to grow beyond limit
	result := val.(int32)
	if result != -1 {
		t.Errorf("grow() = %v, want -1 (failure)", result)
	}
}
