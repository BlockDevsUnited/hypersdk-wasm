package main

/*
#cgo CFLAGS: -I../common
#include "types.h"
#include "callbacks.h"
*/
import "C"

import (
	"context"
	"crypto/rand"
	"errors"
	"os"
	"unsafe"

	"github.com/ava-labs/avalanchego/utils/logging"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/x/contracts/runtime"
	simState "github.com/ava-labs/hypersdk/x/contracts/simulator/state"
)

var (
	ErrInvalidParam = errors.New("invalid parameter")
	SimLogger       = logging.NewLogger("Simulator")
	SimContext      = context.TODO()
)

//export CallContract
func CallContract(statePtr unsafe.Pointer, ctx *C.SimulatorCallContext) C.CallContractResponse {
	if statePtr == nil || ctx == nil {
		return newCallContractResponse(nil, 0, ErrInvalidParam)
	}

	// build the call info
	var contractAddr codec.Address
	copy(contractAddr[:], C.GoBytes(unsafe.Pointer(&ctx.contract_address.address[0]), 33))

	var actorAddr codec.Address
	copy(actorAddr[:], C.GoBytes(unsafe.Pointer(&ctx.actor_address.address[0]), 33))

	state := simState.WrapSimulatorState(statePtr)
	r := runtime.NewRuntime(runtime.NewConfig(), SimLogger)
	callInfo := &runtime.CallInfo{
		State:        state,
		Actor:        actorAddr,
		Contract:     contractAddr,
		FunctionName: C.GoString(ctx.method),
		Params:       C.GoBytes(unsafe.Pointer(ctx.params.data), C.int(ctx.params.length)),
		Fuel:         uint64(ctx.max_gas),
		Height:       uint64(ctx.height),
		Timestamp:    uint64(ctx.timestamp),
	}

	// execute the contract
	result, err := r.WithDefaults(runtime.CallInfo{}).CallContract(SimContext, callInfo)
	return newCallContractResponse(result, callInfo.RemainingFuel(), err)
}

//export CreateContract
func CreateContract(statePtr unsafe.Pointer, path *C.char) C.CreateContractResponse {
	if statePtr == nil || path == nil {
		return C.CreateContractResponse{
			error: C.CString(ErrInvalidParam.Error()),
		}
	}

	// read contract file
	contractPath := C.GoString(path)
	contractBytes, err := os.ReadFile(contractPath)
	if err != nil {
		return C.CreateContractResponse{
			error: C.CString(err.Error()),
		}
	}

	// generate contract ID
	contractID := make(runtime.ContractID, 32)
	if _, err := rand.Read(contractID); err != nil {
		return C.CreateContractResponse{
			error: C.CString(err.Error()),
		}
	}

	// store contract bytes
	state := simState.WrapSimulatorState(statePtr)
	err = state.SetContractBytes(SimContext, contractID, contractBytes)
	if err != nil {
		return C.CreateContractResponse{
			error: C.CString(err.Error()),
		}
	}

	// Create contract ID bytes
	contractIdBytes := C.Bytes{
		data:   (*C.uint8_t)(&contractID[0]),
		length: C.size_t(len(contractID)),
	}

	// Create contract address
	var contractAddr C.Address
	for i := 0; i < len(contractID); i++ {
		contractAddr.address[i] = C.uchar(contractID[i])
	}

	return C.CreateContractResponse{
		contract_id:      contractIdBytes,
		contract_address: contractAddr,
		error:           nil,
	}
}

//export GetBalance
func GetBalance(statePtr unsafe.Pointer, address C.Address) C.uint64_t {
	if statePtr == nil {
		return 0
	}

	state := simState.WrapSimulatorState(statePtr)
	var addr codec.Address
	copy(addr[:], C.GoBytes(unsafe.Pointer(&address.address[0]), 33))

	balance, err := state.GetBalance(SimContext, addr)
	if err != nil {
		return 0
	}
	return C.uint64_t(balance)
}

//export SetBalance
func SetBalance(statePtr unsafe.Pointer, address C.Address, balance C.uint64_t) {
	if statePtr == nil {
		return
	}

	state := simState.WrapSimulatorState(statePtr)
	var addr codec.Address
	copy(addr[:], C.GoBytes(unsafe.Pointer(&address.address[0]), 33))

	_ = state.TransferBalance(SimContext, codec.Address{}, addr, uint64(balance))
}

func newCallContractResponse(result []byte, fuel uint64, err error) C.CallContractResponse {
	var resp C.CallContractResponse
	if err != nil {
		resp.error = C.CString(err.Error())
		return resp
	}

	if len(result) > 0 {
		resp.result.data = (*C.uint8_t)(&result[0])
		resp.result.length = C.size_t(len(result))
	}
	resp.fuel = C.uint64_t(fuel)
	return resp
}

func main() {}
