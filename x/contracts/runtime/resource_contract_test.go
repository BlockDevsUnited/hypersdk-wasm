// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"testing"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/resources"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/testutils"
	"github.com/stretchr/testify/require"
)

func TestResourceContract(t *testing.T) {
	r := require.New(t)
	ctx := context.Background()

	// Create a new resource manager with mock validator
	db := testutils.NewMockDB()
	typeSystem := resources.NewTypeSystem()
	validator := testutils.NewMockValidator()
	manager := resources.NewResourceManager(db, typeSystem, validator)

	// Register the Balance type
	balanceType := core.ResourceType{
		Name:      "Balance",
		Abilities: []core.Ability{core.Key, core.Store},
	}
	err := typeSystem.RegisterType(ctx, balanceType)
	r.NoError(err)

	// Create resource contract
	owner := codec.Address{1}
	contract := NewResourceContract(ctx, owner, manager)

	// Test resource creation
	typ := balanceType
	data := []byte{0, 0, 0, 0, 0, 0, 0, 100} // uint64(100)

	id, err := contract.CreateResource(typ, owner, data)
	r.NoError(err)
	r.NotEmpty(id)

	// Test resource retrieval
	resource, err := contract.GetResource(id)
	r.NoError(err)
	r.Equal(typ.Name, resource.Type().Name)
	r.Equal(owner, resource.Owner())
	r.Equal(data, resource.Data())

	// Test resource update
	newData := []byte{0, 0, 0, 0, 0, 0, 0, 200} // uint64(200)
	err = contract.UpdateResource(id, newData)
	r.NoError(err)

	resource, err = contract.GetResource(id)
	r.NoError(err)
	r.Equal(newData, resource.Data())

	// Test resource transfer
	newOwner := codec.Address{2}
	err = contract.TransferResource(id, newOwner)
	r.NoError(err)

	resource, err = contract.GetResource(id)
	r.NoError(err)
	r.Equal(newOwner, resource.Owner())

	// Test resource deletion
	err = contract.DeleteResource(id)
	r.Error(err) // Should fail since we're not the owner anymore

	// Create new contract as new owner
	newContract := NewResourceContract(ctx, newOwner, manager)
	err = newContract.DeleteResource(id)
	r.NoError(err)

	_, err = contract.GetResource(id)
	r.Error(err) // Resource should be deleted
}
