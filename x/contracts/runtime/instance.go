// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"fmt"

	"github.com/bytecodealliance/wasmtime-go/v25"
)

const (
	// The name of the memory export
	MemoryName = "memory"
	// The name of the allocation function export
	AllocName = "alloc"
)

// EphemeralInstance represents a temporary contract instance
type EphemeralInstance struct {
	inst   *wasmtime.Instance
	store  *wasmtime.Store
	result []byte
}

// NewEphemeralInstance creates a new ephemeral instance from a contract instance
func NewEphemeralInstance(inst *wasmtime.Instance, store *wasmtime.Store) *EphemeralInstance {
	// Configure memory limits
	if mem := inst.GetExport(store, "memory"); mem != nil {
		if memInst := mem.Memory(); memInst != nil {
			// Set maximum memory pages
			_, err := memInst.Grow(store, 0)
			if err != nil {
				// Memory already at maximum or error occurred
				return nil
			}
		}
	}

	return &EphemeralInstance{
		inst:  inst,
		store: store,
	}
}

// Execute executes a contract call with the given parameters
func (ei *EphemeralInstance) Execute(ctx context.Context, callInfo *CallInfo) ([]byte, error) {
	// Set initial fuel for this execution
	if err := ei.store.SetFuel(callInfo.Fuel); err != nil {
		return nil, fmt.Errorf("failed to set initial fuel: %w", err)
	}

	// Create contract instance
	contractInst := &ContractInstance{
		inst:   ei.inst,
		store:  ei.store,
		result: ei.result,
	}

	// Execute the contract
	return contractInst.call(ctx, callInfo)
}

// Close releases all resources associated with this instance
func (ei *EphemeralInstance) Close() {
	if ei.store != nil {
		ei.store.Close()
	}
}

// GetRemainingFuel returns the amount of fuel left for this instance
func (ei *EphemeralInstance) GetRemainingFuel() (uint64, error) {
	return ei.store.GetFuel()
}

// ConsumeFuel consumes the specified amount of fuel
func (ei *EphemeralInstance) ConsumeFuel(fuel uint64) error {
	remaining, err := ei.store.GetFuel()
	if err != nil {
		return err
	}

	if remaining < fuel {
		return fmt.Errorf("insufficient fuel: have %d, need %d", remaining, fuel)
	}

	return ei.store.SetFuel(remaining - fuel)
}

// GetInstance returns the underlying wasmtime instance
func (ei *EphemeralInstance) GetInstance() *wasmtime.Instance {
	return ei.inst
}
