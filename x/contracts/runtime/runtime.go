// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"fmt"
	"reflect"

	"github.com/ava-labs/avalanchego/cache"
	"github.com/ava-labs/avalanchego/utils/logging"
	"github.com/bytecodealliance/wasmtime-go/v25"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/state"
)

type WasmRuntime struct {
	log    logging.Logger
	engine *wasmtime.Engine
	cfg    *Config

	contractCache cache.Cacher[string, *wasmtime.Module]

	callerInfo map[uintptr]*CallInfo
	linker     *wasmtime.Linker
	limits     ResourceLimits
}

type StateManager interface {
	BalanceManager
	ContractManager
}

type BalanceManager interface {
	GetBalance(ctx context.Context, address codec.Address) (uint64, error)
	TransferBalance(ctx context.Context, from codec.Address, to codec.Address, amount uint64) error
}

type ContractManager interface {
	// GetContractState returns the state of the contract at the given address.
	GetContractState(address codec.Address) state.Mutable
	// GetAccountContract returns the contract ID associated with the given account.
	// An account represents a specific instance of a contract.
	GetAccountContract(ctx context.Context, account codec.Address) (ContractID, error)
	// GetContractBytes returns the compiled WASM bytes of the contract with the given ID.
	GetContractBytes(ctx context.Context, contractID ContractID) ([]byte, error)
	// NewAccountWithContract creates a new account that represents a specific instance of a contract.
	NewAccountWithContract(ctx context.Context, contractID ContractID, accountCreationData []byte) (codec.Address, error)
	// SetAccountContract associates the given contract ID with the given account.
	SetAccountContract(ctx context.Context, account codec.Address, contractID ContractID) error
	// SetContractBytes stores the compiled WASM bytes of the contract with the given ID.
	SetContractBytes(ctx context.Context, contractID ContractID, contractBytes []byte) error
}

// ValidationError represents an error that occurs during WebAssembly validation
type ValidationError struct {
	Message string // General error message
	Rule    string // Optional: specific validation rule that failed
	Cause   error  // Optional: underlying error
}

func (e *ValidationError) Error() string {
	if e.Rule != "" {
		if e.Cause != nil {
			return fmt.Sprintf("validation failed for rule %s: %s: %v", e.Rule, e.Message, e.Cause)
		}
		return fmt.Sprintf("validation failed for rule %s: %s", e.Rule, e.Message)
	}
	if e.Cause != nil {
		return fmt.Sprintf("validation error: %s: %v", e.Message, e.Cause)
	}
	return fmt.Sprintf("validation error: %s", e.Message)
}

func (e *ValidationError) Unwrap() error {
	return e.Cause
}

// NewValidationError creates a new ValidationError with an optional rule name
func NewValidationError(msg string, rule string, err error) error {
	return &ValidationError{
		Message: msg,
		Rule:    rule,
		Cause:   err,
	}
}

func NewRuntime(
	cfg *Config,
	log logging.Logger,
) *WasmRuntime {
	hostImports := NewImports()

	runtime := &WasmRuntime{
		log:        log,
		cfg:        cfg,
		engine:     wasmtime.NewEngineWithConfig(cfg.wasmConfig),
		callerInfo: map[uintptr]*CallInfo{},
		contractCache: cache.NewSizedLRU(cfg.ContractCacheSize, func(id string, mod *wasmtime.Module) int {
			bytes, err := mod.Serialize()
			if err != nil {
				panic(err)
			}
			return len(id) + len(bytes)
		}),
		limits: DefaultResourceLimits(),
	}

	// Register contract module first since other modules may depend on it
	hostImports.AddModule(NewContractModule(runtime))
	hostImports.AddModule(NewLogModule())
	hostImports.AddModule(NewBalanceModule())
	hostImports.AddModule(NewStateAccessModule())

	linker, err := hostImports.createLinker(runtime)
	if err != nil {
		panic(err)
	}

	runtime.linker = linker

	return runtime
}

func (r *WasmRuntime) WithDefaults(callInfo CallInfo) CallContext {
	return CallContext{r: r, defaultCallInfo: callInfo}
}

func (r *WasmRuntime) getModule(ctx context.Context, callInfo *CallInfo, id []byte) (*wasmtime.Module, error) {
	// Check cache first
	if mod, ok := r.contractCache.Get(string(id)); ok {
		return mod, nil
	}
	// If not in cache, get bytecode and compile
	contractBytes, err := callInfo.State.GetContractBytes(ctx, id)
	if err != nil {
		return nil, err
	}

	// Validate contract size
	if uint32(len(contractBytes)) > r.limits.MaxContractSize {
		return nil, NewValidationError(
			fmt.Sprintf("contract size %d exceeds maximum allowed size %d", len(contractBytes), r.limits.MaxContractSize),
			"contract-size",
			nil,
		)
	}

	// Parse module to validate structure before compilation
	if err := r.validateModule(contractBytes); err != nil {
		return nil, err
	}

	mod, err := wasmtime.NewModule(r.engine, contractBytes)
	if err != nil {
		return nil, NewValidationError("failed to create module", "", err)
	}
	r.contractCache.Put(string(id), mod)
	return mod, nil
}

// validateModule checks if a WebAssembly module respects resource limits
func (r *WasmRuntime) validateModule(bytes []byte) error {
	// Parse the raw bytes instead of using ParseWat
	mod, err := wasmtime.NewModule(r.engine, bytes)
	if err != nil {
		return NewValidationError("failed to parse module", "", err)
	}

	// Get module info
	exports := mod.Exports()
	imports := mod.Imports()

	// Validate function count
	funcCount := uint32(len(exports) + len(imports))
	if funcCount > r.limits.MaxFunctions {
		return NewValidationError(
			fmt.Sprintf("function count %d exceeds maximum allowed %d", funcCount, r.limits.MaxFunctions),
			"function-count",
			nil,
		)
	}

	// Validate import count
	if uint32(len(imports)) > r.limits.MaxImports {
		return NewValidationError(
			fmt.Sprintf("import count %d exceeds maximum allowed %d", len(imports), r.limits.MaxImports),
			"import-count",
			nil,
		)
	}

	// Validate export count
	if uint32(len(exports)) > r.limits.MaxExports {
		return NewValidationError(
			fmt.Sprintf("export count %d exceeds maximum allowed %d", len(exports), r.limits.MaxExports),
			"export-count",
			nil,
		)
	}

	return nil
}

func (r *WasmRuntime) CallContract(ctx context.Context, callInfo *CallInfo) (result []byte, err error) {
	contractID, err := callInfo.State.GetAccountContract(ctx, callInfo.Contract)
	if err != nil {
		return nil, err
	}
	contractModule, err := r.getModule(ctx, callInfo, contractID)
	if err != nil {
		return nil, err
	}
	inst, err := r.getInstance(contractModule)
	if err != nil {
		return nil, err
	}
	callInfo.inst = inst

	r.setCallInfo(inst.store, callInfo)
	defer r.deleteCallInfo(inst.store)

	return inst.call(ctx, callInfo)
}

func (r *WasmRuntime) getInstance(contractModule *wasmtime.Module) (*ContractInstance, error) {
	store := wasmtime.NewStore(r.engine)
	store.SetEpochDeadline(1)

	inst, err := r.linker.Instantiate(store, contractModule)
	if err != nil {
		return nil, NewValidationError("failed to instantiate module", "", err)
	}
	return &ContractInstance{inst: inst, store: store}, nil
}

func toMapKey(storeLike wasmtime.Storelike) uintptr {
	return reflect.ValueOf(storeLike.Context()).Pointer()
}

func (r *WasmRuntime) setCallInfo(storeLike wasmtime.Storelike, info *CallInfo) {
	r.callerInfo[toMapKey(storeLike)] = info
}

func (r *WasmRuntime) getCallInfo(storeLike wasmtime.Storelike) *CallInfo {
	return r.callerInfo[toMapKey(storeLike)]
}

func (r *WasmRuntime) deleteCallInfo(storeLike wasmtime.Storelike) {
	delete(r.callerInfo, toMapKey(storeLike))
}
