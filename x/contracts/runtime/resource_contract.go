// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"errors"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/resources"
)

var (
	ErrInvalidResourceOperation = errors.New("invalid resource operation")
)

// ResourceContract provides resource management functionality to WebAssembly contracts
type ResourceContract struct {
	ctx     context.Context
	owner   codec.Address
	manager resources.ResourceManager
}

// NewResourceContract creates a new resource contract
func NewResourceContract(ctx context.Context, owner codec.Address, manager resources.ResourceManager) *ResourceContract {
	return &ResourceContract{
		ctx:     ctx,
		owner:   owner,
		manager: manager,
	}
}

// CreateResource creates a new resource
func (r *ResourceContract) CreateResource(typ core.ResourceType, owner codec.Address, data []byte) (core.ResourceID, error) {
	return r.manager.CreateResource(r.ctx, typ, owner, data)
}

// TransferResource transfers a resource to a new owner
func (r *ResourceContract) TransferResource(id core.ResourceID, to codec.Address) error {
	// Get current owner
	owner, err := r.manager.GetOwner(r.ctx, id)
	if err != nil {
		return err
	}

	// Only owner can transfer
	if owner != r.owner {
		return ErrInvalidResourceOperation
	}

	return r.manager.TransferResource(r.ctx, id, owner, to)
}

// UpdateResource updates a resource's data
func (r *ResourceContract) UpdateResource(id core.ResourceID, data []byte) error {
	// Get current owner
	owner, err := r.manager.GetOwner(r.ctx, id)
	if err != nil {
		return err
	}

	// Only owner can update
	if owner != r.owner {
		return ErrInvalidResourceOperation
	}

	return r.manager.UpdateResource(r.ctx, id, data)
}

// DeleteResource deletes a resource
func (r *ResourceContract) DeleteResource(id core.ResourceID) error {
	// Get current owner
	owner, err := r.manager.GetOwner(r.ctx, id)
	if err != nil {
		return err
	}

	// Only owner can delete
	if owner != r.owner {
		return ErrInvalidResourceOperation
	}

	return r.manager.DeleteResource(r.ctx, id)
}

// GetResource returns a resource by ID
func (r *ResourceContract) GetResource(id core.ResourceID) (core.Resource, error) {
	return r.manager.GetResource(r.ctx, id)
}
