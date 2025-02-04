// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package validators

import (
	"github.com/bytecodealliance/wasmtime-go/v25"

	"github.com/ava-labs/hypersdk/x/contracts/runtime"
)

// Common security rules that can be used with any validator

// WithDeterministicFloatingPoint returns a ValidatorOption that configures
// floating point validation rules
func WithDeterministicFloatingPoint() runtime.ValidatorOption {
	return func(v runtime.ModuleValidator) error {
		if dv, ok := v.(*DefaultValidator); ok {
			dv.AddCustomRule(runtime.SecurityRule{
				Type: runtime.RuleTypeFloatingPoint,
				Name: "strict-float",
				AllowList: []string{
					"f32.add", "f32.sub", "f32.mul", "f32.div",
					"f64.add", "f64.sub", "f64.mul", "f64.div",
				},
				DenyList: []string{
					"f32.nearest", "f32.ceil", "f32.floor", "f32.trunc",
					"f64.nearest", "f64.ceil", "f64.floor", "f64.trunc",
				},
			})
		}
		return nil
	}
}

// WithRestrictedInstructions returns a ValidatorOption that configures
// instruction validation rules
func WithRestrictedInstructions() runtime.ValidatorOption {
	return func(v runtime.ModuleValidator) error {
		if dv, ok := v.(*DefaultValidator); ok {
			dv.AddCustomRule(runtime.SecurityRule{
				Type: runtime.RuleTypeInstruction,
				Name: "restricted-instructions",
				DenyList: []string{
					"memory.grow", "memory.size",
					"table.grow", "table.size",
					"unreachable",
				},
			})
		}
		return nil
	}
}

// WithCustomMemoryLimits returns a ValidatorOption that configures
// custom memory validation rules
func WithCustomMemoryLimits(maxPages uint32) runtime.ValidatorOption {
	return func(v runtime.ModuleValidator) error {
		if dv, ok := v.(*DefaultValidator); ok {
			dv.AddCustomRule(runtime.SecurityRule{
				Type: runtime.RuleTypeMemory,
				Name: "custom-memory-limits",
				Validator: func(mod *wasmtime.Module) error {
					for _, exp := range mod.Exports() {
						if exp.Type().MemoryType() != nil {
							memType := exp.Type().MemoryType()
							min := uint32(memType.Minimum())
							if min > maxPages {
								return runtime.NewValidationError(
									"memory-limits",
									"memory pages exceed custom limit",
									runtime.ErrResourceLimitExceeded,
								)
							}
							ok, max := memType.Maximum()
							if ok && uint32(max) > maxPages {
								return runtime.NewValidationError(
									"memory-limits",
									"memory pages exceed custom limit",
									runtime.ErrResourceLimitExceeded,
								)
							}
						}
					}
					return nil
				},
			})
		}
		return nil
	}
}
