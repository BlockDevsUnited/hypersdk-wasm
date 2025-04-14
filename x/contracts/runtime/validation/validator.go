// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package validation

import (
	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
)

// ResourceValidator validates resource operations
type ResourceValidator interface {
	// Type validation
	ValidateResourceType(typ core.ResourceType) error
	
	// Operation validation
	ValidateMove(resource core.Resource, from, to codec.Address) error
	ValidateUpdate(resource core.Resource, newValue []byte) error
}
