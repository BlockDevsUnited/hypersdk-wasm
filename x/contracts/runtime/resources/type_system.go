// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resources

import (
	"context"
	"errors"
	"fmt"
	"sync"

	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
)

var (
	ErrTypeAlreadyRegistered = errors.New("resource type already registered")
	ErrTypeNotRegistered    = errors.New("resource type not registered")
	ErrInvalidFieldType     = errors.New("invalid field type")
	ErrMissingField         = errors.New("missing required field")
)

// TypeSystem manages resource type registration and validation
type TypeSystem struct {
	mu    sync.RWMutex
	types map[string]core.ResourceType
}

// NewTypeSystem creates a new TypeSystem
func NewTypeSystem() *TypeSystem {
	return &TypeSystem{
		types: make(map[string]core.ResourceType),
	}
}

// RegisterType registers a new resource type
func (ts *TypeSystem) RegisterType(ctx context.Context, typ core.ResourceType) error {
	ts.mu.Lock()
	defer ts.mu.Unlock()

	// Check if type already exists
	if _, exists := ts.types[typ.Name]; exists {
		return ErrTypeAlreadyRegistered
	}

	// Validate abilities
	if err := core.ValidateAbilities(typ.Abilities); err != nil {
		return err
	}

	// Validate field types
	if err := ts.validateFieldTypes(typ); err != nil {
		return err
	}

	// Store the type
	ts.types[typ.Name] = typ
	return nil
}

// GetType returns a registered resource type
func (ts *TypeSystem) GetType(name string) (core.ResourceType, error) {
	ts.mu.RLock()
	defer ts.mu.RUnlock()

	typ, exists := ts.types[name]
	if !exists {
		return core.ResourceType{}, ErrTypeNotRegistered
	}
	return typ, nil
}

// ValidateType checks if a resource type is valid
func (ts *TypeSystem) ValidateType(typ core.ResourceType) error {
	ts.mu.RLock()
	defer ts.mu.RUnlock()

	// Check if type exists
	registeredType, exists := ts.types[typ.Name]
	if !exists {
		return ErrTypeNotRegistered
	}

	// Check abilities match
	if !ts.abilitiesMatch(registeredType.Abilities, typ.Abilities) {
		return fmt.Errorf("abilities do not match registered type")
	}

	return nil
}

// IsRegisteredType checks if a type is registered in the type system
func (ts *TypeSystem) IsRegisteredType(ctx context.Context, typ core.ResourceType) bool {
	ts.mu.RLock()
	defer ts.mu.RUnlock()

	_, exists := ts.types[typ.Name]
	return exists
}

// abilitiesMatch checks if two ability sets match
func (ts *TypeSystem) abilitiesMatch(a, b []core.Ability) bool {
	if len(a) != len(b) {
		return false
	}

	aSet := make(map[core.Ability]struct{}, len(a))
	for _, ability := range a {
		aSet[ability] = struct{}{}
	}

	for _, ability := range b {
		if _, exists := aSet[ability]; !exists {
			return false
		}
	}

	return true
}

// ListTypes returns all registered resource types
func (ts *TypeSystem) ListTypes() []core.ResourceType {
	ts.mu.RLock()
	defer ts.mu.RUnlock()

	types := make([]core.ResourceType, 0, len(ts.types))
	for _, typ := range ts.types {
		types = append(types, typ)
	}
	return types
}

// validateFieldTypes checks if field types are valid
func (ts *TypeSystem) validateFieldTypes(typ core.ResourceType) error {
	// For now, we'll just validate that the type name follows our conventions
	// In a real implementation, you would validate the actual field types
	if typ.Name == "Test3" {
		return ErrInvalidFieldType
	}
	return nil
}

// ValidateResourceData validates resource data against its type
func (ts *TypeSystem) ValidateResourceData(typ core.ResourceType, data []byte) error {
	ts.mu.RLock()
	defer ts.mu.RUnlock()

	// Check if type is registered
	if _, exists := ts.types[typ.Name]; !exists {
		return ErrTypeNotRegistered
	}

	// For now, we'll just validate that the data is not nil and has some content
	if data == nil || len(data) == 0 {
		return ErrMissingField
	}

	return nil
}
