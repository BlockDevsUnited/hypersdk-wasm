// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resources

import (
	"context"
	"errors"
	"sync"
	"testing"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/testutils"
	"github.com/stretchr/testify/require"
)

type mockDB struct {
	mu   sync.RWMutex
	data map[string][]byte
}

func newMockDB() *mockDB {
	return &mockDB{
		data: make(map[string][]byte),
	}
}

func (m *mockDB) GetValue(_ context.Context, key []byte) ([]byte, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	
	if value, ok := m.data[string(key)]; ok {
		return value, nil
	}
	return nil, errors.New("not found")
}

func (m *mockDB) Insert(_ context.Context, key []byte, value []byte) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	
	m.data[string(key)] = value
	return nil
}

func (m *mockDB) Remove(_ context.Context, key []byte) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	
	delete(m.data, string(key))
	return nil
}

func setupTest(t *testing.T) (context.Context, ResourceManager, *TypeSystem) {
	ctx := context.Background()
	db := newMockDB()
	ts := NewTypeSystem()
	validator := testutils.NewMockValidator()
	manager := NewResourceManager(db, ts, validator)

	// Register test type
	typ := core.ResourceType{
		Name:      "TestType",
		Abilities: []core.Ability{core.Key, core.Store},
	}
	err := ts.RegisterType(ctx, typ)
	require.NoError(t, err)

	return ctx, manager, ts
}

func TestResourceManager_CreateResource(t *testing.T) {
	r := require.New(t)
	ctx, manager, _ := setupTest(t)

	// Test creating a resource
	owner := codec.Address{1}
	typ := core.ResourceType{
		Name:      "TestType",
		Abilities: []core.Ability{core.Key, core.Store},
	}
	data := []byte("test data")

	id, err := manager.CreateResource(ctx, typ, owner, data)
	r.NoError(err)
	r.NotEmpty(id)

	// Verify resource
	resource, err := manager.GetResource(ctx, id)
	r.NoError(err)
	r.Equal(typ, resource.Type())
	r.Equal(owner, resource.Owner())
	r.Equal(data, resource.Data())
}

func TestResourceManager_TransferResource(t *testing.T) {
	r := require.New(t)
	ctx, manager, _ := setupTest(t)

	// Create a resource first
	owner := codec.Address{1}
	newOwner := codec.Address{2}
	typ := core.ResourceType{
		Name:      "TestType",
		Abilities: []core.Ability{core.Key, core.Store},
	}
	data := []byte("test data")

	id, err := manager.CreateResource(ctx, typ, owner, data)
	r.NoError(err)

	// Test transfer
	err = manager.TransferResource(ctx, id, owner, newOwner)
	r.NoError(err)

	// Verify transfer
	resource, err := manager.GetResource(ctx, id)
	r.NoError(err)
	r.Equal(newOwner, resource.Owner())
	r.True(resource.IsMoved())
}

func TestResourceManager_DeleteResource(t *testing.T) {
	r := require.New(t)
	ctx, manager, _ := setupTest(t)

	// Create a resource first
	owner := codec.Address{1}
	typ := core.ResourceType{
		Name:      "TestType",
		Abilities: []core.Ability{core.Key, core.Store},
	}
	data := []byte("test data")

	id, err := manager.CreateResource(ctx, typ, owner, data)
	r.NoError(err)

	// Test deletion
	err = manager.DeleteResource(ctx, id)
	r.NoError(err)

	// Verify deletion
	_, err = manager.GetResource(ctx, id)
	r.Error(err)
}
