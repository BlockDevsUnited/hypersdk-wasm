// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resources

import (
	"context"
	"crypto/sha256"
	"encoding/binary"
	"fmt"
	"sync"
	"time"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/state"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/errors"
)

// ResourceManager manages resource lifecycle and operations
type ResourceManager interface {
	// Resource operations
	CreateResource(ctx context.Context, typ core.ResourceType, owner codec.Address, data []byte) (core.ResourceID, error)
	TransferResource(ctx context.Context, id core.ResourceID, from, to codec.Address) error
	UpdateResource(ctx context.Context, id core.ResourceID, data []byte) error
	DeleteResource(ctx context.Context, id core.ResourceID) error

	// Resource queries
	GetResource(ctx context.Context, id core.ResourceID) (core.Resource, error)
	GetOwner(ctx context.Context, id core.ResourceID) (codec.Address, error)
	GetResourcesByOwner(ctx context.Context, owner codec.Address) ([]core.Resource, error)
	GetResourcesByType(ctx context.Context, typ core.ResourceType) ([]core.Resource, error)
}

type resourceManager struct {
	mu         sync.RWMutex
	db         state.Mutable
	typeSystem *TypeSystem
	validator  core.ResourceValidator
}

// NewResourceManager creates a new resource manager
func NewResourceManager(db state.Mutable, typeSystem *TypeSystem, validator core.ResourceValidator) ResourceManager {
	return &resourceManager{
		db:         db,
		typeSystem: typeSystem,
		validator:  validator,
	}
}

// CreateResource creates a new resource
func (rm *resourceManager) CreateResource(ctx context.Context, typ core.ResourceType, owner codec.Address, data []byte) (core.ResourceID, error) {
	// Validate resource type first (no lock needed)
	if err := rm.typeSystem.ValidateType(typ); err != nil {
		return core.ResourceID{}, fmt.Errorf("invalid resource type: %w", err)
	}

	// Generate resource ID (no lock needed)
	id := rm.generateResourceID(typ, owner, data)

	// Check if resource exists before acquiring write lock
	exists := false
	rm.mu.RLock()
	_, err := rm.getResourceNoLock(ctx, id)
	if err == nil {
		exists = true
	}
	rm.mu.RUnlock()

	if exists {
		return core.ResourceID{}, errors.ErrResourceAlreadyExists
	}

	// Now acquire write lock for creating the resource
	rm.mu.Lock()
	defer rm.mu.Unlock()

	// Double-check existence under write lock
	_, err = rm.getResourceNoLock(ctx, id)
	if err == nil {
		return core.ResourceID{}, errors.ErrResourceAlreadyExists
	}

	// Create resource
	resource := core.NewBaseResource(id, typ, owner, data)

	// Write resource to state
	if err := rm.writeResource(ctx, resource); err != nil {
		return core.ResourceID{}, fmt.Errorf("failed to write resource: %w", err)
	}

	return id, nil
}

// DeleteResource deletes a resource
func (rm *resourceManager) DeleteResource(ctx context.Context, id core.ResourceID) error {
	rm.mu.Lock()
	defer rm.mu.Unlock()

	return rm.deleteResourceInternal(ctx, id)
}

// GetOwner returns the owner of a resource
func (rm *resourceManager) GetOwner(ctx context.Context, id core.ResourceID) (codec.Address, error) {
	// Get resource data from state directly
	data, err := rm.db.GetValue(ctx, rm.resourceKey(id))
	if err != nil {
		return codec.Address{}, errors.ErrResourceNotFound
	}

	// Unmarshal resource
	resource := &core.BaseResource{}
	if err := resource.Unmarshal(data); err != nil {
		return codec.Address{}, fmt.Errorf("failed to unmarshal resource: %w", err)
	}

	return resource.Owner(), nil
}

// TransferResource transfers a resource from one owner to another
func (rm *resourceManager) TransferResource(ctx context.Context, id core.ResourceID, from, to codec.Address) error {
	// Get current owner first (no lock needed)
	owner, err := rm.GetOwner(ctx, id)
	if err != nil {
		return err
	}

	// Verify current owner
	if owner != from {
		return errors.ErrInvalidTransfer
	}

	// Now acquire write lock for transfer
	rm.mu.Lock()
	defer rm.mu.Unlock()

	// Double-check resource exists and owner hasn't changed
	currentOwner, err := rm.GetOwner(ctx, id)
	if err != nil {
		return err
	}
	if currentOwner != from {
		return errors.ErrInvalidTransfer
	}

	return rm.transferResourceInternal(ctx, id, to)
}

// GetResource returns a resource by ID
func (rm *resourceManager) GetResource(ctx context.Context, id core.ResourceID) (core.Resource, error) {
	rm.mu.RLock()
	defer rm.mu.RUnlock()
	return rm.getResourceNoLock(ctx, id)
}

// getResourceNoLock returns a resource by ID without acquiring a lock
func (rm *resourceManager) getResourceNoLock(ctx context.Context, id core.ResourceID) (core.Resource, error) {
	// Get resource data from state
	data, err := rm.db.GetValue(ctx, rm.resourceKey(id))
	if err != nil {
		return nil, errors.ErrResourceNotFound
	}

	// Unmarshal resource
	resource := &core.BaseResource{}
	if err := resource.Unmarshal(data); err != nil {
		return nil, fmt.Errorf("failed to unmarshal resource: %w", err)
	}

	return resource, nil
}

// UpdateResource updates a resource's data
func (rm *resourceManager) UpdateResource(ctx context.Context, id core.ResourceID, data []byte) error {
	// Get resource first (uses read lock)
	resource, err := rm.GetResource(ctx, id)
	if err != nil {
		return err
	}

	// Validate resource type (no lock needed)
	if err := rm.typeSystem.ValidateType(resource.Type()); err != nil {
		return fmt.Errorf("invalid resource type: %w", err)
	}

	// Now acquire write lock for updating
	rm.mu.Lock()
	defer rm.mu.Unlock()

	// Double-check resource exists under write lock
	_, err = rm.getResourceNoLock(ctx, id)
	if err != nil {
		return err
	}

	return rm.updateResourceInternal(ctx, id, data)
}

// GetResourcesByOwner returns all resources owned by an address
func (rm *resourceManager) GetResourcesByOwner(ctx context.Context, owner codec.Address) ([]core.Resource, error) {
	// TODO: Implement this
	return nil, nil
}

// GetResourcesByType returns all resources of a given type
func (rm *resourceManager) GetResourcesByType(ctx context.Context, typ core.ResourceType) ([]core.Resource, error) {
	// TODO: Implement this
	return nil, nil
}

// Internal helper methods

// writeResource writes a resource to state
func (rm *resourceManager) writeResource(ctx context.Context, resource *core.BaseResource) error {
	data, err := resource.Marshal()
	if err != nil {
		return fmt.Errorf("failed to marshal resource: %w", err)
	}

	return rm.db.Insert(ctx, rm.resourceKey(resource.ID()), data)
}

// deleteResourceInternal deletes a resource from state
func (rm *resourceManager) deleteResourceInternal(ctx context.Context, id core.ResourceID) error {
	return rm.db.Remove(ctx, rm.resourceKey(id))
}

// transferResourceInternal transfers a resource to a new owner
func (rm *resourceManager) transferResourceInternal(ctx context.Context, id core.ResourceID, to codec.Address) error {
	// Get resource without lock since we already have write lock
	resource, err := rm.getResourceNoLock(ctx, id)
	if err != nil {
		return err
	}

	// Create new resource with new owner
	newResource := core.NewBaseResource(id, resource.Type(), to, resource.Data())
	newResource.MarkMoved()

	// Write new resource
	return rm.writeResource(ctx, newResource)
}

// updateResourceInternal updates a resource's data
func (rm *resourceManager) updateResourceInternal(ctx context.Context, id core.ResourceID, newData []byte) error {
	// Get resource without lock since we already have write lock
	resource, err := rm.getResourceNoLock(ctx, id)
	if err != nil {
		return err
	}

	// Create new resource with updated data
	newResource := core.NewBaseResource(id, resource.Type(), resource.Owner(), newData)

	// Write updated resource
	return rm.writeResource(ctx, newResource)
}

// generateResourceID generates a unique resource ID
func (rm *resourceManager) generateResourceID(typ core.ResourceType, owner codec.Address, data []byte) core.ResourceID {
	// Combine type name, owner, and current timestamp
	timestamp := make([]byte, 8)
	binary.BigEndian.PutUint64(timestamp, uint64(time.Now().UnixNano()))

	// Hash the combined data
	hasher := sha256.New()
	hasher.Write([]byte(typ.Name))
	hasher.Write(owner[:])
	hasher.Write(data)
	hasher.Write(timestamp)

	var id core.ResourceID
	copy(id[:], hasher.Sum(nil))
	return id
}

// resourceKey returns the state key for a resource
func (rm *resourceManager) resourceKey(id core.ResourceID) []byte {
	key := make([]byte, len(id)+1)
	key[0] = 'r' // prefix for resource keys
	copy(key[1:], id[:])
	return key
}
