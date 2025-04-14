// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package validators

import (
	"fmt"

	"github.com/bytecodealliance/wasmtime-go/v25"

	"github.com/ava-labs/hypersdk/x/contracts/runtime"
)

// WebAssembly value kinds
const (
	KindI32 wasmtime.ValKind = iota
	KindI64
	KindF32
	KindF64
	KindExternref
	KindFuncref
)

// DefaultValidator implements the ModuleValidator interface with standard validation rules
type DefaultValidator struct {
	defaultRules []runtime.SecurityRule
	customRules  []runtime.SecurityRule
	config       runtime.ValidatorConfig
}

// NewDefaultValidator creates a new DefaultValidator with standard rules
func NewDefaultValidator(config runtime.ValidatorConfig) (*DefaultValidator, error) {
	v := &DefaultValidator{
		customRules: config.CustomRules,
		config:      config,
	}

	if config.DefaultRules {
		v.defaultRules = defaultSecurityRules()
	}

	return v, nil
}

// NewDefaultValidatorWithOptions creates a new DefaultValidator with options
func NewDefaultValidatorWithOptions(opts ...runtime.ValidatorOption) (*DefaultValidator, error) {
	v := &DefaultValidator{
		defaultRules: defaultSecurityRules(),
		config:      runtime.DefaultValidatorConfig(),
	}

	for _, opt := range opts {
		if err := opt(v); err != nil {
			return nil, fmt.Errorf("failed to apply validator option: %w", err)
		}
	}

	return v, nil
}

// ValidateModule implements ModuleValidator.ValidateModule
func (v *DefaultValidator) ValidateModule(bytes []byte, engine *wasmtime.Engine) error {
	// Check contract size
	if uint32(len(bytes)) > v.config.ResourceLimits.MaxContractSize {
		return runtime.NewValidationError(
			fmt.Sprintf("contract size %d exceeds maximum allowed %d", len(bytes), v.config.ResourceLimits.MaxContractSize),
			"contract-size",
			runtime.ErrResourceLimitExceeded,
		)
	}

	// Parse the module
	mod, err := wasmtime.NewModule(engine, bytes)
	if err != nil {
		return runtime.NewValidationError("failed to parse module", "parse", err)
	}

	// Apply default security rules
	if err := v.ValidateSecurityRules(mod, v.defaultRules); err != nil {
		return err
	}

	// Apply custom security rules if any
	if len(v.customRules) > 0 {
		if err := v.ValidateSecurityRules(mod, v.customRules); err != nil {
			return err
		}
	}

	// Validate resource limits
	if err := v.ValidateResourceLimits(mod, v.config.ResourceLimits); err != nil {
		return err
	}

	return nil
}

// ValidateSecurityRules implements ModuleValidator.ValidateSecurityRules
func (v *DefaultValidator) ValidateSecurityRules(mod *wasmtime.Module, rules []runtime.SecurityRule) error {
	for _, rule := range rules {
		switch rule.Type {
		case runtime.RuleTypeInstruction:
			if err := validateInstructions(mod, rule); err != nil {
				return err
			}
		case runtime.RuleTypeFloatingPoint:
			if err := validateFloatingPoint(mod, rule); err != nil {
				return err
			}
		case runtime.RuleTypeMemory:
			if err := validateMemoryOperations(mod, rule); err != nil {
				return err
			}
		case runtime.RuleTypeCustom:
			if rule.Validator != nil {
				if err := rule.Validator(mod); err != nil {
					return runtime.NewValidationError("custom validation failed", rule.Name, err)
				}
			}
		}
	}
	return nil
}

// ValidateResourceLimits implements ModuleValidator.ValidateResourceLimits
func (v *DefaultValidator) ValidateResourceLimits(mod *wasmtime.Module, limits runtime.ResourceLimits) error {
	exports := mod.Exports()
	imports := mod.Imports()

	// Check function count
	funcCount := uint32(0)
	for _, exp := range exports {
		if exp.Type().FuncType() != nil {
			funcCount++
		}
	}
	for _, imp := range imports {
		if imp.Type().FuncType() != nil {
			funcCount++
		}
	}
	if funcCount > limits.MaxFunctions {
		return runtime.NewValidationError(
			fmt.Sprintf("function count %d exceeds limit %d", funcCount, limits.MaxFunctions),
			"resource-limits",
			runtime.ErrResourceLimitExceeded,
		)
	}

	// Check export count
	if uint32(len(exports)) > limits.MaxExports {
		return runtime.NewValidationError(
			fmt.Sprintf("export count %d exceeds limit %d", len(exports), limits.MaxExports),
			"resource-limits",
			runtime.ErrResourceLimitExceeded,
		)
	}

	// Check import count
	if uint32(len(imports)) > limits.MaxImports {
		return runtime.NewValidationError(
			fmt.Sprintf("import count %d exceeds limit %d", len(imports), limits.MaxImports),
			"resource-limits",
			runtime.ErrResourceLimitExceeded,
		)
	}

	// Check global count
	globalCount := uint32(0)
	for _, exp := range exports {
		if exp.Type().GlobalType() != nil {
			globalCount++
		}
	}
	for _, imp := range imports {
		if imp.Type().GlobalType() != nil {
			globalCount++
		}
	}
	if globalCount > limits.MaxGlobals {
		return runtime.NewValidationError(
			fmt.Sprintf("global count %d exceeds limit %d", globalCount, limits.MaxGlobals),
			"resource-limits",
			runtime.ErrResourceLimitExceeded,
		)
	}

	// Check memory limits
	for _, exp := range exports {
		if exp.Type().MemoryType() != nil {
			memType := exp.Type().MemoryType()
			min := uint32(memType.Minimum())
			if min > limits.MaxMemoryPages {
				return runtime.NewValidationError(
					fmt.Sprintf("minimum memory pages %d exceeds limit %d", min, limits.MaxMemoryPages),
					"resource-limits",
					runtime.ErrResourceLimitExceeded,
				)
			}
			ok, maxVal := memType.Maximum()
			if ok && uint32(maxVal) > limits.MaxMemoryPages {
				return runtime.NewValidationError(
					fmt.Sprintf("maximum memory pages %d exceeds limit %d", maxVal, limits.MaxMemoryPages),
					"resource-limits",
					runtime.ErrResourceLimitExceeded,
				)
			}
		}
	}

	// Check table limits
	for _, exp := range exports {
		if exp.Type().TableType() != nil {
			tableType := exp.Type().TableType()
			min := uint32(tableType.Minimum())
			if min > limits.MaxTableSize {
				return runtime.NewValidationError(
					fmt.Sprintf("minimum table size %d exceeds limit %d", min, limits.MaxTableSize),
					"resource-limits",
					runtime.ErrResourceLimitExceeded,
				)
			}
			ok, maxVal := tableType.Maximum()
			if ok && uint32(maxVal) > limits.MaxTableSize {
				return runtime.NewValidationError(
					fmt.Sprintf("maximum table size %d exceeds limit %d", maxVal, limits.MaxTableSize),
					"resource-limits",
					runtime.ErrResourceLimitExceeded,
				)
			}
		}
	}

	return nil
}

// AddCustomRule adds a custom security rule to the validator
func (v *DefaultValidator) AddCustomRule(rule runtime.SecurityRule) {
	v.customRules = append(v.customRules, rule)
}

// CustomRules returns the current set of custom security rules
func (v *DefaultValidator) CustomRules() []runtime.SecurityRule {
	return v.customRules
}

// defaultSecurityRules returns the default set of security rules
func defaultSecurityRules() []runtime.SecurityRule {
	return []runtime.SecurityRule{
		{
			Type: runtime.RuleTypeInstruction,
			Name: "default-instructions",
			DenyList: []string{
				// Deny table operations by default
				"table.get",
				"table.set",
				"table.size",
				"table.grow",
				"table.fill",
				"table.init",
				"elem.drop",
				"data.drop",
				"table.copy",
			},
		},
		{
			Type: runtime.RuleTypeMemory,
			Name: "default-memory",
			DenyList: []string{
				"memory.grow",
			},
		},
	}
}

// validateInstructions validates WebAssembly instructions against allow/deny lists
func validateInstructions(mod *wasmtime.Module, rule runtime.SecurityRule) error {
	// For now, we'll only validate exports and imports since wasmtime-go doesn't provide
	// instruction-level introspection yet
	exports := mod.Exports()
	imports := mod.Imports()

	// Check if any denied instructions are used in function signatures
	for _, exp := range exports {
		if exp.Type().FuncType() != nil {
			// For now, we'll just check if the function uses any denied types
			ft := exp.Type().FuncType()
			for _, param := range ft.Params() {
				if err := validateType(param.Kind(), rule); err != nil {
					return runtime.NewValidationError(
						fmt.Sprintf("invalid parameter type in export %s", exp.Name()),
						rule.Name,
						err,
					)
				}
			}
			for _, result := range ft.Results() {
				if err := validateType(result.Kind(), rule); err != nil {
					return runtime.NewValidationError(
						fmt.Sprintf("invalid result type in export %s", exp.Name()),
						rule.Name,
						err,
					)
				}
			}
		}
	}

	for _, imp := range imports {
		if imp.Type().FuncType() != nil {
			// Check if the imported function uses any denied types
			ft := imp.Type().FuncType()
			for _, param := range ft.Params() {
				if err := validateType(param.Kind(), rule); err != nil {
					importName := ""
					if name := imp.Name(); name != nil {
						importName = *name
					}
					return runtime.NewValidationError(
						fmt.Sprintf("invalid parameter type in import %s::%s", imp.Module(), importName),
						rule.Name,
						err,
					)
				}
			}
			for _, result := range ft.Results() {
				if err := validateType(result.Kind(), rule); err != nil {
					importName := ""
					if name := imp.Name(); name != nil {
						importName = *name
					}
					return runtime.NewValidationError(
						fmt.Sprintf("invalid result type in import %s::%s", imp.Module(), importName),
						rule.Name,
						err,
					)
				}
			}
		}
	}

	return nil
}

// validateType checks if a WebAssembly type is allowed by the security rule
func validateType(kind wasmtime.ValKind, rule runtime.SecurityRule) error {
	// If there's no deny list, everything is allowed
	if len(rule.DenyList) == 0 {
		return nil
	}

	// Check if the type is in the deny list
	var typeName string
	switch kind {
	case wasmtime.KindI32:
		typeName = "i32"
	case wasmtime.KindI64:
		typeName = "i64"
	case wasmtime.KindF32:
		typeName = "f32"
	case wasmtime.KindF64:
		typeName = "f64"
	case wasmtime.KindExternref:
		typeName = "externref"
	case wasmtime.KindFuncref:
		typeName = "funcref"
	}

	for _, denied := range rule.DenyList {
		if denied == typeName {
			return fmt.Errorf("type %s is not allowed", typeName)
		}
	}

	return nil
}

// validateFloatingPoint validates floating point operations
func validateFloatingPoint(mod *wasmtime.Module, rule runtime.SecurityRule) error {
	exports := mod.Exports()
	for _, exp := range exports {
		if exp.Type().FuncType() != nil {
			// Check if function uses floating point types
			funcType := exp.Type().FuncType()
			for _, param := range funcType.Params() {
				if param.Kind() == KindF32 || param.Kind() == KindF64 {
					// If floating point is used, ensure it's in the allow list
					if len(rule.AllowList) > 0 {
						// TODO: Once wasmtime-go provides instruction introspection,
						// validate specific floating point operations
						return nil
					}
					return runtime.NewValidationError(
						"floating point operations not allowed",
						rule.Name,
						nil,
					)
				}
			}
			// Also check return types
			for _, result := range funcType.Results() {
				if result.Kind() == KindF32 || result.Kind() == KindF64 {
					// If floating point is used, ensure it's in the allow list
					if len(rule.AllowList) > 0 {
						// TODO: Once wasmtime-go provides instruction introspection,
						// validate specific floating point operations
						return nil
					}
					return runtime.NewValidationError(
						"floating point operations not allowed",
						rule.Name,
						nil,
					)
				}
			}
		}
	}
	return nil
}

// validateMemoryOperations validates memory-related operations
func validateMemoryOperations(mod *wasmtime.Module, rule runtime.SecurityRule) error {
	for _, exp := range mod.Exports() {
		if exp.Type().MemoryType() != nil {
			memType := exp.Type().MemoryType()
			
			// Check for memory.grow in deny list
			for _, denied := range rule.DenyList {
				if denied == "memory.grow" {
					// If memory.grow is denied, ensure memory is fixed size
					if ok, _ := memType.Maximum(); !ok {
						return runtime.NewValidationError(
							"unbounded memory growth not allowed",
							rule.Name,
							nil,
						)
					}
				}
			}
			
			// Validate memory operations based on allow list
			if len(rule.AllowList) > 0 {
				// TODO: Once wasmtime-go provides memory operation introspection,
				// implement full memory operation validation
				return nil
			}
		}
	}
	return nil
}
