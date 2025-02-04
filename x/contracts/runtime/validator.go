// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"errors"
	"github.com/bytecodealliance/wasmtime-go/v25"
)

var (
	// ErrInvalidModule indicates that a WebAssembly module is invalid
	ErrInvalidModule = errors.New("invalid module")
	// ErrResourceLimitExceeded indicates that a resource limit was exceeded
	ErrResourceLimitExceeded = errors.New("resource limit exceeded")
	// ErrSecurityRuleViolation indicates that a security rule was violated
	ErrSecurityRuleViolation = errors.New("security rule violation")
	// ErrInvalidInstruction indicates that a disallowed instruction was found
	ErrInvalidInstruction = errors.New("invalid instruction")
	// ErrInvalidMemoryOperation indicates that a disallowed memory operation was found
	ErrInvalidMemoryOperation = errors.New("invalid memory operation")
)

// SecurityRuleType defines the type of security rule
type SecurityRuleType string

const (
	// RuleTypeInstruction validates specific WebAssembly instructions
	RuleTypeInstruction SecurityRuleType = "instruction"
	// RuleTypeFloatingPoint validates floating point operations
	RuleTypeFloatingPoint SecurityRuleType = "floating_point"
	// RuleTypeMemory validates memory operations
	RuleTypeMemory SecurityRuleType = "memory"
	// RuleTypeCustom allows for custom validation rules
	RuleTypeCustom SecurityRuleType = "custom"
)

// SecurityRule defines a validation rule for WebAssembly modules
type SecurityRule struct {
	// Type of the security rule
	Type SecurityRuleType
	// Name of the rule for identification
	Name string
	// AllowList contains permitted items (e.g., instructions)
	AllowList []string
	// DenyList contains forbidden items
	DenyList []string
	// Custom validation function for complex rules
	Validator func(mod *wasmtime.Module) error
}

// ValidatorConfig defines configuration options for module validation
type ValidatorConfig struct {
	// ResourceLimits defines the resource constraints for modules
	ResourceLimits ResourceLimits
	// DefaultRules specifies whether to apply default security rules
	DefaultRules bool
	// CustomRules contains additional security rules to apply
	CustomRules []SecurityRule
}

// DefaultValidatorConfig returns a ValidatorConfig with safe defaults
func DefaultValidatorConfig() ValidatorConfig {
	return ValidatorConfig{
		ResourceLimits: DefaultResourceLimits(),
		DefaultRules:   true,
		CustomRules:    []SecurityRule{},
	}
}

// ModuleValidator defines the interface for WebAssembly module validation
type ModuleValidator interface {
	// ValidateModule performs comprehensive validation of a WebAssembly module
	ValidateModule(bytes []byte, engine *wasmtime.Engine) error

	// ValidateSecurityRules applies the given security rules to a module
	ValidateSecurityRules(mod *wasmtime.Module, rules []SecurityRule) error

	// ValidateResourceLimits ensures the module respects resource constraints
	ValidateResourceLimits(mod *wasmtime.Module, limits ResourceLimits) error
}

// ValidatorOption defines a function type for configuring validators
type ValidatorOption func(v ModuleValidator) error
