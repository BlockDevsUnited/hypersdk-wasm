// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"fmt"
	"github.com/bytecodealliance/wasmtime-go/v25"
	"golang.org/x/exp/maps"
	"golang.org/x/exp/slices"
)

var nilResult = []wasmtime.Val{wasmtime.ValI32(0)}

type Imports struct {
	Modules map[string]*ImportModule
}

type ImportModule struct {
	Name          string
	HostFunctions map[string]HostFunction
}

func (i *ImportModule) SetFuelCost(functionName string, fuelCost uint64) bool {
	hostFunction, ok := i.HostFunctions[functionName]
	if ok {
		hostFunction.FuelCost = fuelCost
		i.HostFunctions[functionName] = hostFunction
	}

	return ok
}

func NewImports() *Imports {
	return &Imports{Modules: map[string]*ImportModule{}}
}

func (i *Imports) AddModule(mod *ImportModule) {
	i.Modules[mod.Name] = mod
}

func (i *Imports) SetFuelCost(moduleName string, functionName string, fuelCost uint64) bool {
	if module, ok := i.Modules[moduleName]; ok {
		return module.SetFuelCost(functionName, fuelCost)
	}

	return false
}

func (i *Imports) Clone() *Imports {
	return &Imports{
		Modules: maps.Clone(i.Modules),
	}
}

func (i *Imports) createLinker(r *WasmRuntime) (*wasmtime.Linker, error) {
	linker := wasmtime.NewLinker(r.engine)
	if r.log != nil {
		r.log.Debug("creating linker for imports")
	}

	// Set memory configuration
	memoryType := wasmtime.NewMemoryType(
		uint32(r.limits.MaxInitialMemoryPages),
		true, // shared
		uint32(r.limits.MaxMemoryPages), // Convert to uint32
	)
	store := wasmtime.NewStore(r.engine)
	memory, err := wasmtime.NewMemory(store, memoryType)
	if err != nil {
		return nil, NewValidationError("failed to create memory", "", err)
	}
	err = linker.DefineWasi()
	if err != nil {
		return nil, NewValidationError("failed to define WASI", "", err)
	}

	// Register memory first
	err = linker.Define(store, "", "memory", memory)
	if err != nil {
		return nil, NewValidationError("failed to define memory", "", err)
	}

	// Then register all module functions
	for moduleName, module := range i.Modules {
		if r.log != nil {
			r.log.Debug(fmt.Sprintf("registering module: %s", moduleName))
		}
		
		for funcName, hostFunc := range module.HostFunctions {
			if r.log != nil {
				r.log.Debug(fmt.Sprintf("registering function %s::%s", 
					moduleName, funcName))
			}
				
			err := linker.FuncNew(moduleName, funcName,
				hostFunc.Function.wasmType(),
				hostFunc.convert(r))
			if err != nil {
				return nil, fmt.Errorf("failed to create function %s::%s: %w", 
					moduleName, funcName, err)
			}
		}
	}

	return linker, nil
}

type HostFunction struct {
	Function HostFunctionType
	FuelCost uint64
}

func (f HostFunction) convert(r *WasmRuntime) func(*wasmtime.Caller, []wasmtime.Val) ([]wasmtime.Val, *wasmtime.Trap) {
	return func(caller *wasmtime.Caller, vals []wasmtime.Val) ([]wasmtime.Val, *wasmtime.Trap) {
		callInfo := r.getCallInfo(caller)
		if err := callInfo.ConsumeFuel(f.FuelCost); err != nil {
			return nil, convertToTrap(err)
		}
		return f.Function.call(callInfo, caller, vals)
	}
}

type HostFunctionType interface {
	wasmType() *wasmtime.FuncType
	call(*CallInfo, *wasmtime.Caller, []wasmtime.Val) ([]wasmtime.Val, *wasmtime.Trap)
}

var typeI32 = wasmtime.NewValType(wasmtime.KindI32)
var typeI64 = wasmtime.NewValType(wasmtime.KindI64)

type Function[T any, U any] func(*CallInfo, T) (U, error)

func (Function[T, U]) wasmType() *wasmtime.FuncType {
	return wasmtime.NewFuncType([]*wasmtime.ValType{typeI32, typeI32}, []*wasmtime.ValType{typeI32})
}

func (f Function[T, U]) call(callInfo *CallInfo, caller *wasmtime.Caller, vals []wasmtime.Val) ([]wasmtime.Val, *wasmtime.Trap) {
	input, err := getInputFromMemory[T](caller, vals)
	if err != nil {
		return writeOutputToMemory[interface{}](callInfo, nil, err)
	}
	results, err := f(callInfo, *input)
	return writeOutputToMemory(callInfo, results, err)
}

type FunctionNoInput[T any] func(*CallInfo) (T, error)

func (FunctionNoInput[T]) wasmType() *wasmtime.FuncType {
	return wasmtime.NewFuncType([]*wasmtime.ValType{}, []*wasmtime.ValType{typeI32})
}

func (f FunctionNoInput[T]) call(callInfo *CallInfo, _ *wasmtime.Caller, _ []wasmtime.Val) ([]wasmtime.Val, *wasmtime.Trap) {
	results, err := f(callInfo)
	return writeOutputToMemory[T](callInfo, results, err)
}

type FunctionNoOutput[T any] func(*CallInfo, T) error

func (FunctionNoOutput[T]) wasmType() *wasmtime.FuncType {
	return wasmtime.NewFuncType([]*wasmtime.ValType{typeI32, typeI32}, []*wasmtime.ValType{})
}

func (f FunctionNoOutput[T]) call(callInfo *CallInfo, caller *wasmtime.Caller, vals []wasmtime.Val) ([]wasmtime.Val, *wasmtime.Trap) {
	input, err := getInputFromMemory[T](caller, vals)
	if err != nil {
		return []wasmtime.Val{}, convertToTrap(err)
	}
	err = f(callInfo, *input)
	return []wasmtime.Val{}, convertToTrap(err)
}

func getInputFromMemory[T any](caller *wasmtime.Caller, vals []wasmtime.Val) (*T, error) {
	offset := vals[0].I32()
	length := vals[1].I32()

	if offset == 0 || length == 0 {
		return new(T), nil
	}
	return Deserialize[T](caller.GetExport(MemoryName).Memory().UnsafeData(caller)[offset : offset+length])
}

func writeOutputToMemory[T any](callInfo *CallInfo, results T, err error) ([]wasmtime.Val, *wasmtime.Trap) {
	if err != nil {
		return nilResult, convertToTrap(err)
	}
	data, err := Serialize(results)
	if data == nil || err != nil {
		return nilResult, convertToTrap(err)
	}
	offset, err := callInfo.inst.writeToMemory(data)
	if err != nil {
		return nilResult, convertToTrap(err)
	}
	return []wasmtime.Val{wasmtime.ValI32(offset)}, nil
}

// functionFromWasmVals is a helper to create a HostFunctionType from a function that directly handles wasmtime.Val values
func functionFromWasmVals(f func(store *wasmtime.Store, callInfo *CallInfo, args []wasmtime.Val) ([]wasmtime.Val, error)) HostFunctionType {
	return &directWasmValsFunc{f: f}
}

// directWasmValsFunc implements HostFunctionType for functions that directly handle wasmtime.Val values
type directWasmValsFunc struct {
	f func(store *wasmtime.Store, callInfo *CallInfo, args []wasmtime.Val) ([]wasmtime.Val, error)
}

func (f *directWasmValsFunc) wasmType() *wasmtime.FuncType {
	// Function type for set_call_result: func(i32, i32) -> void (no return value)
	return wasmtime.NewFuncType([]*wasmtime.ValType{typeI32, typeI32}, []*wasmtime.ValType{})
}

func (f *directWasmValsFunc) call(callInfo *CallInfo, caller *wasmtime.Caller, args []wasmtime.Val) ([]wasmtime.Val, *wasmtime.Trap) {
	// Use the store directly from callInfo
	res, err := f.f(callInfo.inst.store, callInfo, args)
	if err != nil {
		return nilResult, convertToTrap(err)
	}
	return res, nil
}

type simpleValueFunc struct {
	typeFunc func() *wasmtime.FuncType
	callFunc func(*CallInfo, *wasmtime.Caller, []wasmtime.Val) ([]wasmtime.Val, *wasmtime.Trap)
}

func (f *simpleValueFunc) wasmType() *wasmtime.FuncType {
	return f.typeFunc()
}

func (f *simpleValueFunc) call(callInfo *CallInfo, caller *wasmtime.Caller, args []wasmtime.Val) ([]wasmtime.Val, *wasmtime.Trap) {
	return f.callFunc(callInfo, caller, args)
}

// Creates a new environment module with basic functions
func NewEnvModule() *ImportModule {
	return &ImportModule{
		Name: "env",
		HostFunctions: map[string]HostFunction{
			"set_call_result": {
				FuelCost: 100, 
				Function: functionFromWasmVals(func(store *wasmtime.Store, callInfo *CallInfo, args []wasmtime.Val) ([]wasmtime.Val, error) {
					// Extract pointer and length from arguments
					ptr := args[0].I32()
					length := args[1].I32()
					
					// Get memory and read bytes
					mem := callInfo.inst.inst.GetExport(store, MemoryName).Memory()
					data := mem.UnsafeData(store)[ptr:ptr+length]
					
					// Clone the bytes to avoid issues if the WebAssembly memory is reused
					callInfo.inst.result = slices.Clone(data)
					
					// No return value for this function
					return nil, nil
				}),
			},
			"get_call_value": {
				FuelCost: 10, // Low fuel cost for a simple getter
				Function: &simpleValueFunc{
					typeFunc: func() *wasmtime.FuncType {
						// Function type for get_call_value: func() -> i64
						return wasmtime.NewFuncType([]*wasmtime.ValType{}, []*wasmtime.ValType{typeI64})
					},
					callFunc: func(callInfo *CallInfo, caller *wasmtime.Caller, args []wasmtime.Val) ([]wasmtime.Val, *wasmtime.Trap) {
						// Return the Value field from the CallInfo struct
						return []wasmtime.Val{wasmtime.ValI64(int64(callInfo.Value))}, nil
					},
				},
			},
		},
	}
}
