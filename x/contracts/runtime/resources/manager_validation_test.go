// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resources

import (
	"context"
	"errors"
	"testing"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/testutils"
	"github.com/stretchr/testify/require"
)

func TestResourceManagerValidation(t *testing.T) {
	r := require.New(t)
	ctx := context.Background()

	// Create a new resource manager with mock validator
	db := testutils.NewMockDB()
	typeSystem := NewTypeSystem()
	validator := testutils.NewMockValidator()
	manager := NewResourceManager(db, typeSystem, validator)

	// Register valid resource type
	balanceType := core.ResourceType{
		Name:      "Balance",
		Abilities: []core.Ability{core.Key, core.Store},
	}
	err := typeSystem.RegisterType(ctx, balanceType)
	r.NoError(err)

	t.Run("CreateResource", func(t *testing.T) {
		// Set up validator behavior
		invalidTypeErr := errors.New("invalid type")
		validator.ValidateTypeFunc = func(typ core.ResourceType) error {
			if len(typ.Abilities) == 0 {
				return invalidTypeErr
			}
			return nil
		}

		// Valid resource creation
		owner := codec.Address{1}
		data := []byte{0, 0, 0, 0, 0, 0, 0, 100} // uint64(100)

		id, err := manager.CreateResource(ctx, balanceType, owner, data)
		r.NoError(err)
		r.NotEmpty(id)

		// Invalid resource type (no abilities)
		invalidType := core.ResourceType{
			Name: "Invalid",
		}
		_, err = manager.CreateResource(ctx, invalidType, owner, data)
		r.Error(err)
		r.Contains(err.Error(), "invalid resource type")
	})

	t.Run("TransferResource", func(t *testing.T) {
		// Set up validator behavior
		movedErr := errors.New("invalid resource transfer")
		validator.ValidateMoveFunc = func(resource core.Resource, from, to codec.Address) error {
			if resource.IsMoved() {
				return movedErr
			}
			return nil
		}

		// Create a resource to transfer
		owner := codec.Address{1}
		data := []byte{0, 0, 0, 0, 0, 0, 0, 100} // uint64(100)

		id, err := manager.CreateResource(ctx, balanceType, owner, data)
		r.NoError(err)

		// Valid transfer
		newOwner := codec.Address{2}
		err = manager.TransferResource(ctx, id, owner, newOwner)
		r.NoError(err)

		// Invalid transfer (resource already moved)
		err = manager.TransferResource(ctx, id, owner, newOwner)
		r.Error(err)
		r.Contains(err.Error(), "invalid resource transfer")
	})

	t.Run("UpdateResource", func(t *testing.T) {
		// Set up validator behavior
		validator.ValidateUpdateFunc = func(resource core.Resource, newValue []byte) error {
			return nil
		}

		// Create a resource to update
		owner := codec.Address{1}
		data := []byte{0, 0, 0, 0, 0, 0, 0, 100} // uint64(100)

		id, err := manager.CreateResource(ctx, balanceType, owner, data)
		r.NoError(err)

		// Valid update
		newData := []byte{0, 0, 0, 0, 0, 0, 0, 200} // uint64(200)
		err = manager.UpdateResource(ctx, id, newData)
		r.NoError(err)
	})
}
