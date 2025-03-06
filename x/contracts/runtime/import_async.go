// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"fmt"

	"github.com/bytecodealliance/wasmtime-go/v25"
	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/hypersdk/codec"
)

const (
	executeAsyncCost       = 1000
	isOperationCompleteCost = 500
	getAsyncResultCost     = 500
)

// NewAsyncModule creates a new import module for async operations
func NewAsyncModule(r *WasmRuntime) *ImportModule {
	return &ImportModule{
		Name: "async",
		HostFunctions: map[string]HostFunction{
			"execute_async": {
				FuelCost: executeAsyncCost,
				Function: functionFromWasmValsWithType(
					func(store *wasmtime.Store, callInfo *CallInfo, args []wasmtime.Val) ([]wasmtime.Val, error) {
						if callInfo == nil {
							return nil, fmt.Errorf("no call info found")
						}

						// Deduct gas for the operation
						if err := callInfo.ConsumeFuel(executeAsyncCost); err != nil {
							return nil, fmt.Errorf("out of fuel: %w", err)
						}

						// Get the async state manager
						asyncManager := r.GetAsyncStateManager()
						
						// Register a new async operation
						result := asyncManager.RegisterResult()
						
						// In a real implementation, we would store a reference to the function pointer
						// to be executed when the operation completes. For now, just return the ID.
						r.log.Debug(fmt.Sprintf("registered async operation: %s", result.ID))
						
						// Return the operation ID as a 64-bit integer 
						// In a real scenario, we would represent this differently
						id := result.ID
						// Convert the ID to an integer for simplicity in tests
						// In production code, we would need a better ID system
						var idInt int64
						_, err := fmt.Sscanf(id, "%d", &idInt)
						if err != nil {
							return nil, fmt.Errorf("failed to parse ID %s: %w", id, err)
						}
						
						return []wasmtime.Val{wasmtime.ValI64(idInt)}, nil
					},
					[]*wasmtime.ValType{typeI32}, // Input parameter types (function pointer)
					[]*wasmtime.ValType{typeI64}, // Return type (operation ID)
				),
			},
			"is_operation_complete": {
				FuelCost: isOperationCompleteCost,
				Function: functionFromWasmValsWithType(
					func(store *wasmtime.Store, callInfo *CallInfo, args []wasmtime.Val) ([]wasmtime.Val, error) {
						if callInfo == nil {
							return nil, fmt.Errorf("no call info found")
						}

						// Deduct gas for the operation
						if err := callInfo.ConsumeFuel(isOperationCompleteCost); err != nil {
							return nil, fmt.Errorf("out of fuel: %w", err)
						}

						// Get the async state manager
						asyncManager := r.GetAsyncStateManager()
						
						// Get the operation ID from args
						opIDInt := args[0].I64()
						
						// Convert the integer ID to a string
						opID := fmt.Sprintf("%d", opIDInt)
						
						// Get the result
						result := asyncManager.GetResult(opID)
						if result == nil {
							r.log.Debug(fmt.Sprintf("operation not found: %s", opID))
							return []wasmtime.Val{wasmtime.ValI32(0)}, nil
						}
						
						if result.Ready {
							return []wasmtime.Val{wasmtime.ValI32(1)}, nil
						}
						
						return []wasmtime.Val{wasmtime.ValI32(0)}, nil
					},
					[]*wasmtime.ValType{typeI64}, // Input parameter type (operation ID)
					[]*wasmtime.ValType{typeI32}, // Return type (1 if complete, 0 if not)
				),
			},
			"get_async_result": {
				FuelCost: getAsyncResultCost,
				Function: functionFromWasmValsWithType(
					func(store *wasmtime.Store, callInfo *CallInfo, args []wasmtime.Val) ([]wasmtime.Val, error) {
						if callInfo == nil {
							return nil, fmt.Errorf("no call info found")
						}

						// Deduct gas for the operation
						if err := callInfo.ConsumeFuel(getAsyncResultCost); err != nil {
							return nil, fmt.Errorf("out of fuel: %w", err)
						}

						// Get the async state manager
						asyncManager := r.GetAsyncStateManager()
						
						// Get the operation ID from args
						opIDInt := args[0].I64()
						resultPtr := args[1].I32()
						resultLen := args[2].I32()
						
						// Convert the integer ID to a string
						opID := fmt.Sprintf("%d", opIDInt)
						
						// Get the result
						result := asyncManager.GetResult(opID)
						if result == nil {
							r.log.Debug(fmt.Sprintf("operation not found: %s", opID))
							return []wasmtime.Val{wasmtime.ValI32(0)}, nil
						}
						
						if !result.Ready {
							r.log.Debug(fmt.Sprintf("operation not ready: %s", opID))
							return []wasmtime.Val{wasmtime.ValI32(0)}, nil
						}

						// Get the memory for storing the result
						memory := callInfo.inst.inst.GetExport(store, "memory").Memory()
						
						// Check if there's enough memory allocated for the result
						if int(resultLen) < len(result.Value) {
							r.log.Debug(fmt.Sprintf("not enough memory: id=%s, resultLen=%d, valueLen=%d", 
								opID, int(resultLen), len(result.Value)))
							return []wasmtime.Val{wasmtime.ValI32(0)}, nil
						}
						
						// Copy the result into the memory
						memoryData := memory.UnsafeData(store)
						copy(memoryData[uint32(resultPtr):uint32(resultPtr)+uint32(len(result.Value))], result.Value)
						
						return []wasmtime.Val{wasmtime.ValI32(1)}, nil
					},
					[]*wasmtime.ValType{typeI64, typeI32, typeI32}, // Input parameters (operation ID, result ptr, result len)
					[]*wasmtime.ValType{typeI32},                  // Return type (1 if successful, 0 if not)
				),
			},
		},
	}
}

// RegisterAsyncModule registers the async module with the host imports
func RegisterAsyncModule(r *WasmRuntime, imports *Imports) {
	imports.AddModule(NewAsyncModule(r))
	
	// Register the async module in the environment namespace
	envImports := imports.Modules["env"]
	if envImports == nil {
		envImports = &ImportModule{
			Name: "env",
			HostFunctions: map[string]HostFunction{},
		}
		imports.AddModule(envImports)
	}

	// Add trace and set_call_result methods to the env namespace if they don't exist
	if _, exists := envImports.HostFunctions["trace"]; !exists {
		envImports.HostFunctions["trace"] = HostFunction{
			FuelCost: 10, // Low cost for logging
			Function: functionFromWasmValsWithType(
				func(store *wasmtime.Store, callInfo *CallInfo, args []wasmtime.Val) ([]wasmtime.Val, error) {
					// Extract pointer and length
					ptr := args[0].I32()
					length := args[1].I32()
					
					// Get memory
					mem := callInfo.inst.inst.GetExport(store, "memory").Memory()
					memoryData := mem.UnsafeData(store)
					
					// Read bytes
					data := memoryData[ptr:ptr+length]
					
					// Print the data as a string
					fmt.Printf("TRACE: %s\n", string(data))
					
					// Return no value (void)
					return nil, nil
				},
				[]*wasmtime.ValType{typeI32, typeI32}, 
				nil, // No return values
			),
		}
	}

	if _, exists := envImports.HostFunctions["set_call_result"]; !exists {
		envImports.HostFunctions["set_call_result"] = HostFunction{
			FuelCost: 10, // Low cost for setting result
			Function: functionFromWasmValsWithType(
				func(store *wasmtime.Store, callInfo *CallInfo, args []wasmtime.Val) ([]wasmtime.Val, error) {
					// Extract pointer and length
					ptr := args[0].I32()
					length := args[1].I32()
					
					// Get memory
					mem := callInfo.inst.inst.GetExport(store, "memory").Memory()
					memoryData := mem.UnsafeData(store)
					
					// Read bytes
					data := make([]byte, length)
					copy(data, memoryData[ptr:ptr+length])
					
					// Store the result in the instance
					callInfo.inst.result = data
					
					// Return void (no return value)
					return nil, nil
				},
				[]*wasmtime.ValType{typeI32, typeI32},
				nil, // No return values
			),
		}
	}
	
	// Add store_state function if it doesn't exist
	if _, exists := envImports.HostFunctions["store_state"]; !exists {
		envImports.HostFunctions["store_state"] = HostFunction{
			FuelCost: 50, // Medium cost for state access
			Function: functionFromWasmValsWithType(
				func(store *wasmtime.Store, callInfo *CallInfo, args []wasmtime.Val) ([]wasmtime.Val, error) {
					// Extract key pointer and length
					keyPtr := args[0].I32()
					keyLen := args[1].I32()
					// Extract value pointer and length
					valuePtr := args[2].I32()
					valueLen := args[3].I32()
					
					// Get memory
					mem := callInfo.inst.inst.GetExport(store, "memory").Memory()
					memoryData := mem.UnsafeData(store)
					
					// Read key and value
					key := memoryData[keyPtr:keyPtr+keyLen]
					value := memoryData[valuePtr:valuePtr+valueLen]
					
					fmt.Printf("STORE_STATE: key=%s, value=%x (len=%d)\n", string(key), value, len(value))
					
					// Store in state
					ctx := context.Background()
					contractState := callInfo.State.GetContractState(callInfo.Contract)
					var err error
					if len(value) == 0 {
						err = contractState.Remove(ctx, key)
					} else {
						err = contractState.Insert(ctx, key, value)
					}
					
					// Return result (0 for success, non-zero for error)
					if err != nil {
						fmt.Printf("STORE_STATE ERROR: %v\n", err)
						return []wasmtime.Val{wasmtime.ValI32(-1)}, nil
					}
					
					return []wasmtime.Val{wasmtime.ValI32(0)}, nil
				},
				[]*wasmtime.ValType{typeI32, typeI32, typeI32, typeI32},
				[]*wasmtime.ValType{typeI32}, // Returns i32 status code
			),
		}
	}
	
	// Add deploy_contract function if it doesn't exist
	if _, exists := envImports.HostFunctions["deploy_contract"]; !exists {
		envImports.HostFunctions["deploy_contract"] = HostFunction{
			FuelCost: 5000, // High cost for contract deployment
			Function: functionFromWasmValsWithType(
				func(store *wasmtime.Store, callInfo *CallInfo, args []wasmtime.Val) ([]wasmtime.Val, error) {
					// Extract code pointer and length
					codePtr := args[0].I32()
					codeLen := args[1].I32()
					// Extract init data pointer and length
					initPtr := args[2].I32()
					initLen := args[3].I32()
					
					// Get memory
					mem := callInfo.inst.inst.GetExport(store, "memory").Memory()
					memoryData := mem.UnsafeData(store)
					
					// Read code and init data
					code := memoryData[codePtr:codePtr+codeLen]
					initData := memoryData[initPtr:initPtr+initLen]
					
					fmt.Printf("DEPLOY_CONTRACT: code_len=%d, init_data_len=%d\n", len(code), len(initData))
					
					// Generate a contract ID
					contractID := ids.GenerateTestID()
					
					// For testing, create an address with type 0 (contract type)
					address := codec.CreateAddress(0, contractID)
					
					// Store the result for retrieval
					result := make([]byte, 33) // Address is 33 bytes (1 byte type + 32 bytes ID)
					result[0] = 0              // Type 0 for contract address
					copy(result[1:], contractID[:])
					
					// Store the result in the instance
					callInfo.inst.result = result
					
					fmt.Printf("DEPLOY_CONTRACT: Generated address: %x\n", address)
					
					// Return success (address length)
					return []wasmtime.Val{wasmtime.ValI32(33)}, nil
				},
				[]*wasmtime.ValType{typeI32, typeI32, typeI32, typeI32},
				[]*wasmtime.ValType{typeI32}, // Returns i32 status/length
			),
		}
	}
	
	// Add get_value function if it doesn't exist
	if _, exists := envImports.HostFunctions["get_value"]; !exists {
		envImports.HostFunctions["get_value"] = HostFunction{
			FuelCost: 10, // Low cost for getting result
			Function: functionFromWasmValsWithType(
				func(store *wasmtime.Store, callInfo *CallInfo, args []wasmtime.Val) ([]wasmtime.Val, error) {
					// Extract buffer pointer and capacity
					bufPtr := args[0].I32()
					capacity := args[1].I32()
					
					// Get memory
					mem := callInfo.inst.inst.GetExport(store, "memory").Memory()
					memoryData := mem.UnsafeData(store)
					
					// Check if result exists
					if callInfo.inst.result == nil || len(callInfo.inst.result) == 0 {
						fmt.Println("GET_VALUE: No result available")
						return []wasmtime.Val{wasmtime.ValI32(0)}, nil
					}
					
					// Calculate how much we can copy
					copyLen := int32(len(callInfo.inst.result))
					if copyLen > capacity {
						fmt.Printf("GET_VALUE: Buffer capacity %d too small for result %d bytes\n", capacity, copyLen)
						copyLen = capacity
					}
					
					// Copy result to the provided buffer
					copy(memoryData[bufPtr:bufPtr+copyLen], callInfo.inst.result[:copyLen])
					
					fmt.Printf("GET_VALUE: Copied %d bytes to buffer\n", copyLen)
					
					// Return the number of bytes copied
					return []wasmtime.Val{wasmtime.ValI32(copyLen)}, nil
				},
				[]*wasmtime.ValType{typeI32, typeI32},
				[]*wasmtime.ValType{typeI32}, // Returns i32 length
			),
		}
	}
	
	// Add functions from the env module if they don't exist
	if _, exists := envImports.HostFunctions["random_bytes"]; !exists {
		// Import the random_bytes implementation from the env module
		envModule := NewEnvModule()
		if randomBytes, ok := envModule.HostFunctions["random_bytes"]; ok {
			envImports.HostFunctions["random_bytes"] = randomBytes
		}
	}
}
