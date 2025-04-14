// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resources

import (
	"testing"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
	"github.com/stretchr/testify/require"
)

func TestBaseResource(t *testing.T) {
	r := require.New(t)

	id := core.ResourceID{1}
	typ := core.ResourceType{
		Name:      "Test",
		Abilities: []core.Ability{core.Key, core.Store},
	}
	owner := codec.Address{2}
	data := []byte{3}

	resource := NewBaseResource(id, typ, owner, data)
	r.Equal(id, resource.ID())
	r.Equal(typ.Name, resource.Type().Name)
	r.Equal(owner, resource.Owner())
	r.False(resource.IsMoved())
	r.Equal(data, resource.Data())

	resource.MarkMoved()
	r.True(resource.IsMoved())

	// Test marshaling
	marshaled, err := resource.Marshal()
	r.NoError(err)
	r.NotEmpty(marshaled)

	// Test unmarshaling
	newResource := &BaseResource{}
	err = newResource.Unmarshal(marshaled)
	r.NoError(err)
	r.Equal(resource.ID(), newResource.ID())
	r.Equal(resource.Type().Name, newResource.Type().Name)
	r.Equal(resource.Owner(), newResource.Owner())
	r.Equal(resource.IsMoved(), newResource.IsMoved())
	r.Equal(resource.Data(), newResource.Data())
}

type marshalTestCase struct {
	name     string
	resource *BaseResource
}

func TestResourceMarshaling(t *testing.T) {
	r := require.New(t)

	testCases := []marshalTestCase{
		{
			name: "basic resource",
			resource: &BaseResource{
				id: core.ResourceID{1, 2, 3},
				typ: core.ResourceType{
					Name:      "TestType",
					Abilities: []core.Ability{core.Key, core.Store},
				},
				ownerAddr: codec.Address{4, 5, 6},
				data:     []byte{7, 8, 9},
			},
		},
		{
			name: "moved resource",
			resource: &BaseResource{
				id: core.ResourceID{1, 2, 3},
				typ: core.ResourceType{
					Name:      "TestType",
					Abilities: []core.Ability{core.Key, core.Store},
				},
				ownerAddr: codec.Address{4, 5, 6},
				moved:     true,
				data:     []byte{7, 8, 9},
			},
		},
		{
			name: "empty data",
			resource: &BaseResource{
				id: core.ResourceID{1, 2, 3},
				typ: core.ResourceType{
					Name:      "TestType",
					Abilities: []core.Ability{core.Key, core.Store},
				},
				ownerAddr: codec.Address{4, 5, 6},
				data:     []byte{},
			},
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			// Marshal
			marshaled, err := tc.resource.Marshal()
			r.NoError(err)
			r.NotEmpty(marshaled)

			// Unmarshal
			newResource := &BaseResource{}
			err = newResource.Unmarshal(marshaled)
			r.NoError(err)

			// Compare fields
			r.Equal(tc.resource.ID(), newResource.ID())
			r.Equal(tc.resource.Type().Name, newResource.Type().Name)
			r.Equal(tc.resource.Owner(), newResource.Owner())
			r.Equal(tc.resource.IsMoved(), newResource.IsMoved())
			r.Equal(tc.resource.Data(), newResource.Data())
		})
	}
}

func TestResourceMarshalingInvalid(t *testing.T) {
	r := require.New(t)

	// Test unmarshaling invalid data
	resource := &BaseResource{}
	err := resource.Unmarshal([]byte{})
	r.Error(err)
	r.Equal(core.ErrInvalidResourceData, err)

	err = resource.Unmarshal([]byte{1, 2, 3})
	r.Error(err)
	r.Equal(core.ErrInvalidResourceData, err)
}
