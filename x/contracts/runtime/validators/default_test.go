// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package validators

import (
	"fmt"
	"testing"

	"github.com/bytecodealliance/wasmtime-go/v25"
	"github.com/stretchr/testify/require"

	"github.com/ava-labs/hypersdk/x/contracts/runtime"
)

func TestDefaultValidator(t *testing.T) {
	// Test WAT code for various scenarios
	tests := []struct {
		name    string
		wat     string
		opts    []runtime.ValidatorOption
		wantErr bool
		errType error
	}{
		{
			name: "valid simple module",
			wat: `(module
				(func (export "test")
					i32.const 42
					return
				)
			)`,
			wantErr: false,
		},
		{
			name: "valid floating point",
			wat: `(module
				(func (export "test") (result f32)
					f32.const 1
					f32.const 2
					f32.add
				)
			)`,
			wantErr: false,
		},
		{
			name: "invalid memory growth",
			wat: `(module
				(memory 1)
				(func (export "test")
					memory.grow
				)
			)`,
			opts: []runtime.ValidatorOption{
				WithRestrictedInstructions(),
			},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			engine := wasmtime.NewEngine()
			wasm, err := wasmtime.Wat2Wasm(tt.wat)
			require.NoError(t, err)

			validator, err := NewDefaultValidatorWithOptions(tt.opts...)
			require.NoError(t, err)

			err = validator.ValidateModule(wasm, engine)
			if tt.wantErr {
				require.Error(t, err)
				var valErr *runtime.ValidationError
				require.ErrorAs(t, err, &valErr)
				if tt.errType != nil {
					require.ErrorIs(t, valErr.Cause, tt.errType)
				}
			} else {
				require.NoError(t, err)
			}
		})
	}
}

func TestResourceLimits(t *testing.T) {
	tests := []struct {
		name    string
		wat     string
		limits  runtime.ResourceLimits
		wantErr bool
		errType error
	}{
		{
			name: "within limits",
			wat: `(module
				(func (export "test1") nop)
				(func (export "test2") nop)
			)`,
			limits: runtime.ResourceLimits{
				MaxFunctions:     10,
				MaxExports:       10,
				MaxImports:       10,
				MaxGlobals:       10,
				MaxMemoryPages:   10,
				MaxTableSize:     1000,
				MaxContractSize:  1 * 1024 * 1024, // 1MB
			},
			wantErr: false,
		},
		{
			name: "exceeds function limit",
			wat: `(module
				(func (export "test1") nop)
				(func (export "test2") nop)
				(func (export "test3") nop)
			)`,
			limits: runtime.ResourceLimits{
				MaxFunctions:    2,
				MaxExports:      10,
				MaxImports:      10,
				MaxContractSize: 1 * 1024 * 1024,
			},
			wantErr: true,
			errType: runtime.ErrResourceLimitExceeded,
		},
		{
			name: "exceeds export limit",
			wat: `(module
				(func (export "test1") nop)
				(func (export "test2") nop)
			)`,
			limits: runtime.ResourceLimits{
				MaxFunctions:    10,
				MaxExports:      1,
				MaxImports:      10,
				MaxContractSize: 1 * 1024 * 1024,
			},
			wantErr: true,
			errType: runtime.ErrResourceLimitExceeded,
		},
		{
			name: "exceeds global limit",
			wat: `(module
				(global (export "g1") i32 (i32.const 1))
				(global (export "g2") i32 (i32.const 2))
				(global (export "g3") i32 (i32.const 3))
			)`,
			limits: runtime.ResourceLimits{
				MaxFunctions:    10,
				MaxExports:      10,
				MaxImports:      10,
				MaxGlobals:      2,
				MaxContractSize: 1 * 1024 * 1024,
			},
			wantErr: true,
			errType: runtime.ErrResourceLimitExceeded,
		},
		{
			name: "exceeds memory limit",
			wat: `(module
				(memory (export "mem") 5 10)
			)`,
			limits: runtime.ResourceLimits{
				MaxFunctions:    10,
				MaxExports:      10,
				MaxImports:      10,
				MaxGlobals:      10,
				MaxMemoryPages:  4,
				MaxContractSize: 1 * 1024 * 1024,
			},
			wantErr: true,
			errType: runtime.ErrResourceLimitExceeded,
		},
		{
			name: "exceeds table limit",
			wat: `(module
				(table (export "table") 1000 2000 funcref)
			)`,
			limits: runtime.ResourceLimits{
				MaxFunctions:    10,
				MaxExports:      10,
				MaxImports:      10,
				MaxGlobals:      10,
				MaxMemoryPages:  10,
				MaxTableSize:    500,
				MaxContractSize: 1 * 1024 * 1024,
			},
			wantErr: true,
			errType: runtime.ErrResourceLimitExceeded,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			engine := wasmtime.NewEngine()
			wasm, err := wasmtime.Wat2Wasm(tt.wat)
			require.NoError(t, err)

			mod, err := wasmtime.NewModule(engine, wasm)
			require.NoError(t, err)

			validator, err := NewDefaultValidator(runtime.ValidatorConfig{
				ResourceLimits: tt.limits,
				DefaultRules:   true,
			})
			require.NoError(t, err)

			err = validator.ValidateResourceLimits(mod, tt.limits)
			if tt.wantErr {
				require.Error(t, err)
				var valErr *runtime.ValidationError
				require.ErrorAs(t, err, &valErr)
				if tt.errType != nil {
					require.ErrorIs(t, valErr.Cause, tt.errType)
				}
			} else {
				require.NoError(t, err)
			}
		})
	}
}

func TestContractSizeValidation(t *testing.T) {
	tests := []struct {
		name    string
		wat     string
		wantErr bool
		errType error
	}{
		{
			name: "small contract",
			wat: `(module
				(func (export "test") nop)
			)`,
			wantErr: false,
		},
		{
			name:    "large contract",
			wat:     generateLargeWat(),
			wantErr: true,
			errType: runtime.ErrResourceLimitExceeded,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			engine := wasmtime.NewEngine()
			wasm, err := wasmtime.Wat2Wasm(tt.wat)
			require.NoError(t, err)

			validator, err := NewDefaultValidator(runtime.ValidatorConfig{
				ResourceLimits: runtime.DefaultResourceLimits(),
				DefaultRules:   true,
			})
			require.NoError(t, err)

			err = validator.ValidateModule(wasm, engine)
			if tt.wantErr {
				require.Error(t, err)
				var valErr *runtime.ValidationError
				require.ErrorAs(t, err, &valErr)
				if tt.errType != nil {
					require.ErrorIs(t, valErr.Cause, tt.errType)
				}
			} else {
				require.NoError(t, err)
			}
		})
	}
}

func generateLargeWat() string {
	// Generate a WAT module with many functions to exceed size limit
	const numFuncs = 10000
	wat := "(module\n"
	for i := 0; i < numFuncs; i++ {
		wat += fmt.Sprintf("(func (export \"test%d\") (param i32) (result i32) local.get 0)\n", i)
	}
	wat += ")"
	return wat
}
