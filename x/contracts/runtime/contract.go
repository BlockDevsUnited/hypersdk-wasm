// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"errors"
	"fmt"
	"io"
	"bytes"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/bytecodealliance/wasmtime-go/v25"
	"github.com/near/borsh-go"

	"github.com/ava-labs/hypersdk/codec"
)

type ContractID []byte

type CallInfo struct {
	// the state that the contract will run against
	State StateManager

	// the address that originated the initial contract call
	Actor codec.Address

	// the name of the function within the contract that is being called
	FunctionName string

	Contract codec.Address

	// the serialized parameters that will be passed to the called function
	Params []byte

	// the maximum amount of fuel allowed to be consumed by wasm for this call
	Fuel uint64

	// the height of the chain that this call was made from
	Height uint64

	// the timestamp of the chain at the time this call was made
	Timestamp uint64

	// the action id that triggered this call
	ActionID ids.ID

	Value uint64

	inst *ContractInstance
}

func (c *CallInfo) RemainingFuel() uint64 {
	remaining, err := c.inst.store.GetFuel()
	if err != nil {
		return c.Fuel
	}

	return remaining
}

func (c *CallInfo) AddFuel(fuel uint64) {
	// only errors if fuel isn't enable, which it always will be
	remaining, err := c.inst.store.GetFuel()
	if err != nil {
		return
	}

	_ = c.inst.store.SetFuel(remaining + fuel)
}

func (c *CallInfo) ConsumeFuel(fuel uint64) error {
	remaining, err := c.inst.store.GetFuel()
	if err != nil {
		return err
	}

	if remaining < fuel {
		return errors.New("out of fuel")
	}

	err = c.inst.store.SetFuel(remaining - fuel)

	return err
}

type ContractInstance struct {
	inst   *wasmtime.Instance
	store  *wasmtime.Store
	result []byte
}

type ContractContext struct {
	Contract  codec.Address
	Actor     codec.Address
	Height    uint64
	Timestamp uint64
	ActionID  ids.ID
}

func (c ContractContext) customSerialize(b io.Writer) error {
	if err := borsh.NewEncoder(b).Encode(c.Contract); err != nil {
		return err
	}
	if err := borsh.NewEncoder(b).Encode(c.Actor); err != nil {
		return err
	}
	if err := borsh.NewEncoder(b).Encode(c.Height); err != nil {
		return err
	}
	if err := borsh.NewEncoder(b).Encode(c.Timestamp); err != nil {
		return err
	}
	// Convert ids.ID to []byte for serialization
	actionIDBytes := c.ActionID[:]
	_, err := b.Write(actionIDBytes)
	return err
}

func (p *ContractInstance) call(ctx context.Context, callInfo *CallInfo) ([]byte, error) {
	remaining, err := p.store.GetFuel()
	if err != nil {
		return nil, err
	}

	if err := p.store.SetFuel(remaining + callInfo.Fuel); err != nil {
		return nil, err
	}

	if callInfo.Value > 0 {
		if err := callInfo.State.TransferBalance(ctx, callInfo.Actor, callInfo.Contract, callInfo.Value); err != nil {
			return nil, err
		}
	}

	// create the contract context
	contractCtx := ContractContext{
		Contract:  callInfo.Contract,
		Actor:     callInfo.Actor,
		Height:    callInfo.Height,
		Timestamp: callInfo.Timestamp,
		ActionID:  callInfo.ActionID,
	}
	paramsBytes := new(bytes.Buffer)
	if err := contractCtx.customSerialize(paramsBytes); err != nil {
		return nil, err
	}
	
	// Ensure we have at least some parameters to avoid empty key issues
	if len(callInfo.Params) == 0 {
		// Add a placeholder parameter to prevent empty keys in contract storage operations
		// This will be ignored by the contract if not expected
		fmt.Println("DEBUG: Adding placeholder parameter to prevent empty key issues")
		callInfo.Params = []byte{0x01} // Adding a minimal non-empty parameter
	}
	
	paramsBytes.Write(callInfo.Params)

	// Log parameter details for debugging
	fmt.Printf("DEBUG: Parameter details - context size: %d, params size: %d, total size: %d\n", 
		paramsBytes.Len()-len(callInfo.Params), len(callInfo.Params), paramsBytes.Len())

	// copy params into store linear memory
	paramsOffset, err := p.writeToMemory(paramsBytes.Bytes())
	if err != nil {
		return nil, err
	}

	// Enhanced debugging for function name resolution
	fmt.Printf("DEBUG: Calling function: %s (trying exports with prefixes 'export_' or 'wasm_', or without prefix)\n", callInfo.FunctionName)
	
	// WebAssembly exported functions use a prefix
	wasmFunctionName := "export_" + callInfo.FunctionName
	function := p.inst.GetFunc(p.store, wasmFunctionName)
	if function == nil {
		// Try without any prefix - direct export
		function = p.inst.GetFunc(p.store, callInfo.FunctionName)
		if function != nil {
			fmt.Printf("DEBUG: Found direct export for %s (no prefix)\n", callInfo.FunctionName)
		} else {
			// Backward compatibility check with old wasm_ prefix if needed
			wasmFunctionName = "wasm_" + callInfo.FunctionName
			function = p.inst.GetFunc(p.store, wasmFunctionName)
			if function == nil {
				return nil, fmt.Errorf("function %s does not exist (tried prefixes: 'export_', none, 'wasm_')", callInfo.FunctionName)
			}
			fmt.Printf("DEBUG: Found wasm_ prefixed export for %s\n", callInfo.FunctionName)
		}
	} else {
		fmt.Printf("DEBUG: Found export_ prefixed export for %s\n", callInfo.FunctionName)
	}
	
	_, err = function.Call(p.store, paramsOffset)
	if err != nil {
		return nil, fmt.Errorf("function call error (%s): %w", callInfo.FunctionName, err)
	}

	// Enhanced debug logging to print the raw result bytes
	fmt.Printf("DEBUG: Result from %s (len=%d): %v\n", callInfo.FunctionName, len(p.result), p.result)
	if len(p.result) > 0 {
		fmt.Printf("DEBUG: First byte (type ID): 0x%02x\n", p.result[0])
	}

	return p.result, nil
}

func (p *ContractInstance) writeToMemory(data []byte) (int32, error) {
	allocFn := p.inst.GetExport(p.store, AllocName).Func()
	if allocFn == nil {
		return 0, fmt.Errorf("allocation function %s not found", AllocName)
	}

	contractMemory := p.inst.GetExport(p.store, MemoryName).Memory()
	if contractMemory == nil {
		return 0, fmt.Errorf("memory %s not found", MemoryName)
	}

	dataOffsetIntf, err := allocFn.Call(p.store, int32(len(data)))
	if err != nil {
		return 0, err
	}
	dataOffset := dataOffsetIntf.(int32)
	linearMem := contractMemory.UnsafeData(p.store)
	copy(linearMem[dataOffset:], data)
	return dataOffset, nil
}
