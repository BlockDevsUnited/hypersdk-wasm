// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resource_rules

import (
	"encoding/binary"
	"testing"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
	"github.com/stretchr/testify/require"
)

// testResource implements core.Resource for testing
type testResource struct {
	typ    core.ResourceType
	id     core.ResourceID
	owner  codec.Address
	moved  bool
	data   []byte
	amount uint64
}

func (r *testResource) Type() core.ResourceType {
	return r.typ
}

func (r *testResource) ID() core.ResourceID {
	return r.id
}

func (r *testResource) Owner() codec.Address {
	return r.owner
}

func (r *testResource) IsMoved() bool {
	return r.moved
}

func (r *testResource) MarkMoved() {
	r.moved = true
}

func (r *testResource) Data() []byte {
	return r.data
}

func (r *testResource) Marshal() ([]byte, error) {
	return r.data, nil
}

func (r *testResource) Unmarshal(data []byte) error {
	r.data = data
	return nil
}

func TestResourceRuleValidator(t *testing.T) {
	r := require.New(t)

	// Create validator with test rules
	rules := []ResourceOperationRule{
		{
			Type: core.ResourceType{
				Name:      "Balance",
				Abilities: []core.Ability{core.Key, core.Store},
			},
			AllowedOperations: []string{"transfer", "update"},
		},
	}
	validator := NewResourceRuleValidator(rules)

	t.Run("ValidateResourceType", func(t *testing.T) {
		// Valid type
		typ := core.ResourceType{
			Name:      "Balance",
			Abilities: []core.Ability{core.Key, core.Store},
		}
		err := validator.ValidateResourceType(typ)
		r.NoError(err)

		// Invalid type (no name)
		invalidType := core.ResourceType{
			Abilities: []core.Ability{core.Key},
		}
		err = validator.ValidateResourceType(invalidType)
		r.Error(err)

		// Invalid type (no abilities)
		invalidType = core.ResourceType{
			Name: "Invalid",
		}
		err = validator.ValidateResourceType(invalidType)
		r.Error(err)

		// Invalid type (invalid ability)
		invalidType = core.ResourceType{
			Name:      "Invalid",
			Abilities: []core.Ability{"invalid"},
		}
		err = validator.ValidateResourceType(invalidType)
		r.Error(err)
	})

	t.Run("ValidateMove", func(t *testing.T) {
		// Create test resource
		from := codec.Address{1}
		resource := &testResource{
			typ: core.ResourceType{
				Name:      "Balance",
				Abilities: []core.Ability{core.Key, core.Store},
			},
			id:     core.ResourceID{1},
			owner:  from,
			moved:  false,
			data:   []byte{1, 2, 3},
			amount: 100,
		}

		// Valid move
		err := validator.ValidateMove(resource, from, codec.Address{2})
		r.NoError(err)

		// Invalid move (already moved)
		resource.moved = true
		err = validator.ValidateMove(resource, from, codec.Address{2})
		r.Error(err)
	})

	t.Run("ValidateUpdate", func(t *testing.T) {
		// Create test resource
		from := codec.Address{1}
		resource := &testResource{
			typ: core.ResourceType{
				Name:      "Balance",
				Abilities: []core.Ability{core.Key, core.Store},
			},
			id:     core.ResourceID{1},
			owner:  from,
			moved:  false,
			data:   []byte{1, 2, 3},
			amount: 100,
		}

		// Valid update
		err := validator.ValidateUpdate(resource, []byte{4, 5, 6})
		r.NoError(err)

		// Invalid update (moved resource)
		resource.moved = true
		err = validator.ValidateUpdate(resource, []byte{4, 5, 6})
		r.Error(err)
	})
}

func TestBalanceRuleValidation(t *testing.T) {
	r := require.New(t)

	t.Run("ValidateBalanceTransfer", func(t *testing.T) {
		// Create balance resource with properly encoded amount
		from := codec.Address{1}
		amount := uint64(100)
		data := make([]byte, 8)
		binary.BigEndian.PutUint64(data, amount)

		resource := &testResource{
			typ: core.ResourceType{
				Name:      "Balance",
				Abilities: []core.Ability{core.Key, core.Store},
			},
			id:     core.ResourceID{1},
			owner:  from,
			moved:  false,
			data:   data,
			amount: amount,
		}

		// Create validator with balance rules
		rules := []ResourceOperationRule{
			{
				Type: core.ResourceType{
					Name:      "Balance",
					Abilities: []core.Ability{core.Key, core.Store},
				},
				AllowedOperations: []string{"transfer", "update"},
				CustomValidators: []func(core.Resource) error{
					ValidateBalanceNotNegative,
				},
			},
		}
		validator := NewResourceRuleValidator(rules)

		// Valid transfer
		err := validator.ValidateMove(resource, from, codec.Address{2})
		r.NoError(err)

		// Invalid transfer (zero balance)
		zeroData := make([]byte, 8)
		resource.data = zeroData
		resource.amount = 0
		err = validator.ValidateMove(resource, from, codec.Address{2})
		r.Error(err)
		r.ErrorIs(err, ErrUnauthorizedMove)
		r.Contains(err.Error(), "negative balance")
	})
}
