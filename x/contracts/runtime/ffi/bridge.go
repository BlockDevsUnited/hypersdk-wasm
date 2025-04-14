package main

/*
#include <stdlib.h>
#include <stdint.h>
#include <string.h>

typedef unsigned char byte;
typedef struct { byte data[32]; } contract_id_t;
*/
import "C"
import (
	"encoding/json"
	"sync"
	"unsafe"

	"github.com/bytecodealliance/wasmtime-go"
)

var (
	runtime *Runtime
	mu      sync.Mutex
)

type Config struct {
	MaxMemory      int   `json:"max_memory"`
	MaxFuel        int64 `json:"max_fuel"`
	MaxStackHeight int   `json:"max_stack_height"`
}

type Runtime struct {
	config     Config
	contracts  map[[32]byte][]byte
	states     map[[32]byte][]byte
	engine     *wasmtime.Engine
}

func main() {} // Required for c-shared build mode

//export hypersdk_init_runtime
func hypersdk_init_runtime(configC *C.char) C.int {
	mu.Lock()
	defer mu.Unlock()

	if runtime != nil {
		return 0 // Already initialized
	}

	configJSON := C.GoString(configC)
	var config Config
	if err := json.Unmarshal([]byte(configJSON), &config); err != nil {
		return -1
	}

	// Create wasmtime engine with config
	engine := wasmtime.NewEngine()

	runtime = &Runtime{
		config:    config,
		contracts: make(map[[32]byte][]byte),
		states:    make(map[[32]byte][]byte),
		engine:    engine,
	}

	return 0
}

//export hypersdk_deploy_contract
func hypersdk_deploy_contract(wasmBytes *C.byte, wasmLen C.size_t, contractIDOut *C.contract_id_t) C.int {
	if runtime == nil {
		return -1
	}

	// Convert wasm bytes to Go slice
	bytes := C.GoBytes(unsafe.Pointer(wasmBytes), C.int(wasmLen))

	// Validate WASM module
	_, err := wasmtime.NewModule(runtime.engine, bytes)
	if err != nil {
		return -2
	}

	// Generate contract ID (using first 32 bytes as ID for now)
	var id [32]byte
	copy(id[:], bytes[:32])

	// Store contract
	runtime.contracts[id] = bytes

	// Copy ID to output parameter
	C.memcpy(unsafe.Pointer(&contractIDOut.data[0]), unsafe.Pointer(&id[0]), 32)

	return 0
}

//export hypersdk_call_contract
func hypersdk_call_contract(
	contractID *C.contract_id_t,
	functionC *C.char,
	argsBytes *C.byte,
	argsLen C.size_t,
	resultPtr **C.byte,
	resultLen *C.size_t,
) C.int {
	if runtime == nil {
		return -1
	}

	// Convert contract ID to Go array
	var id [32]byte
	C.memcpy(unsafe.Pointer(&id[0]), unsafe.Pointer(&contractID.data[0]), 32)

	function := C.GoString(functionC)
	args := C.GoBytes(unsafe.Pointer(argsBytes), C.int(argsLen))

	// Get contract
	wasmBytes, ok := runtime.contracts[id]
	if !ok {
		return -2
	}

	// Create execution context
	store := wasmtime.NewStore(runtime.engine)
	module, err := wasmtime.NewModule(runtime.engine, wasmBytes)
	if err != nil {
		return -3
	}

	// Execute contract
	instance, err := wasmtime.NewInstance(store, module, []wasmtime.AsExtern{})
	if err != nil {
		return -4
	}

	// Get function
	f := instance.GetExport(store, function).Func()
	if f == nil {
		return -5
	}

	// Call function
	val, err := f.Call(store, args)
	if err != nil {
		return -6
	}

	// Convert result to bytes
	result := val.([]byte)

	// Allocate memory for result
	resultSize := C.size_t(len(result))
	resultBuf := C.malloc(resultSize)
	
	// Copy result to C buffer
	C.memcpy(unsafe.Pointer(resultBuf), unsafe.Pointer(&result[0]), resultSize)

	*resultPtr = (*C.byte)(resultBuf)
	*resultLen = resultSize

	return 0
}

//export hypersdk_get_state
func hypersdk_get_state(
	contractID *C.contract_id_t,
	statePtr **C.byte,
	stateLen *C.size_t,
) C.int {
	if runtime == nil {
		return -1
	}

	// Convert contract ID to Go array
	var id [32]byte
	C.memcpy(unsafe.Pointer(&id[0]), unsafe.Pointer(&contractID.data[0]), 32)

	// Get state
	state, ok := runtime.states[id]
	if !ok {
		// Return empty state if not found
		state = make([]byte, 0)
	}

	// Allocate memory for state
	stateSize := C.size_t(len(state))
	stateBuf := C.malloc(stateSize)
	
	// Copy state to C buffer
	C.memcpy(unsafe.Pointer(stateBuf), unsafe.Pointer(&state[0]), stateSize)

	*statePtr = (*C.byte)(stateBuf)
	*stateLen = stateSize

	return 0
}

//export hypersdk_free_buffer
func hypersdk_free_buffer(ptr unsafe.Pointer) {
	C.free(ptr)
}
