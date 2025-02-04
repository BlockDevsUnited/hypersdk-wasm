// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package testutils

import (
	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
)

// MockValidator is a mock implementation of core.ResourceValidator for testing
type MockValidator struct {
	ValidateTypeFunc func(core.ResourceType) error
	ValidateMoveFunc func(core.Resource, codec.Address, codec.Address) error
	ValidateUpdateFunc func(core.Resource, []byte) error
}

func NewMockValidator() *MockValidator {
	return &MockValidator{
		ValidateTypeFunc: func(core.ResourceType) error { return nil },
		ValidateMoveFunc: func(core.Resource, codec.Address, codec.Address) error { return nil },
		ValidateUpdateFunc: func(core.Resource, []byte) error { return nil },
	}
}

func (m *MockValidator) ValidateResourceType(typ core.ResourceType) error {
	return m.ValidateTypeFunc(typ)
}

func (m *MockValidator) ValidateMove(resource core.Resource, from, to codec.Address) error {
	return m.ValidateMoveFunc(resource, from, to)
}

func (m *MockValidator) ValidateUpdate(resource core.Resource, newValue []byte) error {
	return m.ValidateUpdateFunc(resource, newValue)
}
