// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"testing"

	"github.com/bytecodealliance/wasmtime-go/v25"
	"github.com/stretchr/testify/require"
)

func TestValidationError(t *testing.T) {
	tests := []struct {
		name     string
		message  string
		rule     string
		err      error
		expected string
	}{
		{
			name:    "error with underlying cause",
			message: "validation failed",
			rule:    "test-rule",
			err:     ErrInvalidModule,
			expected: "validation failed for rule test-rule: validation failed: invalid module",
		},
		{
			name:    "error without cause",
			message: "validation failed",
			rule:    "test-rule",
			err:     nil,
			expected: "validation failed for rule test-rule: validation failed",
		},
		{
			name:    "error without rule",
			message: "validation failed",
			rule:    "",
			err:     ErrInvalidModule,
			expected: "validation error: validation failed: invalid module",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := NewValidationError(tt.message, tt.rule, tt.err)
			require.Equal(t, tt.expected, err.Error())
		})
	}
}

func TestSecurityRule(t *testing.T) {
	// Test WAT code for a simple module
	wat := `(module
		(memory 1)
		(func (export "test") (result f32)
			f32.const 1
			f32.const 2
			f32.add
		)
	)`

	engine := wasmtime.NewEngine()
	wasm, err := wasmtime.Wat2Wasm(wat)
	require.NoError(t, err)

	mod, err := wasmtime.NewModule(engine, wasm)
	require.NoError(t, err)

	tests := []struct {
		name    string
		rule    SecurityRule
		wantErr bool
	}{
		{
			name: "valid floating point rule",
			rule: SecurityRule{
				Type:      RuleTypeFloatingPoint,
				Name:      "test-float",
				AllowList: []string{"f32.add"},
			},
			wantErr: false,
		},
		{
			name: "invalid floating point rule",
			rule: SecurityRule{
				Type:      RuleTypeFloatingPoint,
				Name:      "test-float",
				DenyList:  []string{"f32.add"},
			},
			wantErr: true,
		},
		{
			name: "valid custom rule",
			rule: SecurityRule{
				Type: RuleTypeCustom,
				Name: "test-custom",
				Validator: func(mod *wasmtime.Module) error {
					return nil
				},
			},
			wantErr: false,
		},
		{
			name: "invalid custom rule",
			rule: SecurityRule{
				Type: RuleTypeCustom,
				Name: "test-custom",
				Validator: func(mod *wasmtime.Module) error {
					return ErrInvalidModule
				},
			},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.rule.Validator != nil {
				err := tt.rule.Validator(mod)
				if tt.wantErr {
					require.ErrorIs(t, err, ErrInvalidModule)
				} else {
					require.NoError(t, err)
				}
			}
		})
	}
}
