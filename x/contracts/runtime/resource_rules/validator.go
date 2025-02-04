// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resource_rules

import (
	"errors"
	"fmt"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
)

var (
	ErrInvalidResourceType = errors.New("invalid resource type")
	ErrUnauthorizedMove   = errors.New("unauthorized resource move")
	ErrInvalidUpdate      = errors.New("invalid resource update")
)

// ResourceOperationRule defines validation rules for a resource type
type ResourceOperationRule struct {
	Type              core.ResourceType
	AllowedOperations []string
	CustomValidators  []func(core.Resource) error
}

// ruleValidator implements core.ResourceValidator
type ruleValidator struct {
	rules map[string]ResourceOperationRule
}

// NewResourceRuleValidator creates a new validator with the given rules
func NewResourceRuleValidator(rules []ResourceOperationRule) core.ResourceValidator {
	v := &ruleValidator{
		rules: make(map[string]ResourceOperationRule),
	}
	
	// Register rules
	for _, rule := range rules {
		v.rules[rule.Type.Name] = rule
	}
	
	return v
}

// ValidateResourceType validates a resource type definition
func (v *ruleValidator) ValidateResourceType(typ core.ResourceType) error {
	// Check if type has a name
	if typ.Name == "" {
		return fmt.Errorf("%w: empty type name", ErrInvalidResourceType)
	}
	
	// Check if type has abilities
	if len(typ.Abilities) == 0 {
		return fmt.Errorf("%w: no abilities defined", ErrInvalidResourceType)
	}
	
	// Validate abilities
	for _, ability := range typ.Abilities {
		switch ability {
		case core.Key, core.Store, core.Drop:
			// Valid ability
		default:
			return fmt.Errorf("%w: invalid ability %s", ErrInvalidResourceType, ability)
		}
	}
	
	// If there are rules for this type, validate against them
	if rule, exists := v.rules[typ.Name]; exists {
		// Run custom validators if any
		for _, validator := range rule.CustomValidators {
			if err := validator(nil); err != nil {
				return fmt.Errorf("%w: %s", ErrInvalidResourceType, err)
			}
		}
	}
	
	return nil
}

// ValidateMove validates a resource move operation
func (v *ruleValidator) ValidateMove(resource core.Resource, from, to codec.Address) error {
	// Check if resource exists
	if resource == nil {
		return fmt.Errorf("%w: resource does not exist", ErrUnauthorizedMove)
	}
	
	// Check if resource is already moved
	if resource.IsMoved() {
		return fmt.Errorf("%w: resource already moved", ErrUnauthorizedMove)
	}
	
	// Get type rules
	rule, exists := v.rules[resource.Type().Name]
	if exists {
		// Check if move is allowed
		moveAllowed := false
		for _, op := range rule.AllowedOperations {
			if op == "transfer" {
				moveAllowed = true
				break
			}
		}
		if !moveAllowed {
			return fmt.Errorf("%w: move not allowed for type %s", ErrUnauthorizedMove, resource.Type().Name)
		}
		
		// Run custom validators
		for _, validator := range rule.CustomValidators {
			if err := validator(resource); err != nil {
				return fmt.Errorf("%w: %s", ErrUnauthorizedMove, err)
			}
		}
	}
	
	return nil
}

// ValidateUpdate validates a resource update operation
func (v *ruleValidator) ValidateUpdate(resource core.Resource, newValue []byte) error {
	// Check if resource exists
	if resource == nil {
		return fmt.Errorf("%w: resource does not exist", ErrInvalidUpdate)
	}
	
	// Check if resource is moved
	if resource.IsMoved() {
		return fmt.Errorf("%w: cannot update moved resource", ErrInvalidUpdate)
	}
	
	// Get type rules
	rule, exists := v.rules[resource.Type().Name]
	if exists {
		// Check if update is allowed
		updateAllowed := false
		for _, op := range rule.AllowedOperations {
			if op == "update" {
				updateAllowed = true
				break
			}
		}
		if !updateAllowed {
			return fmt.Errorf("%w: update not allowed for type %s", ErrInvalidUpdate, resource.Type().Name)
		}
		
		// Run custom validators
		for _, validator := range rule.CustomValidators {
			if err := validator(resource); err != nil {
				return fmt.Errorf("%w: %s", ErrInvalidUpdate, err)
			}
		}
	}
	
	return nil
}
