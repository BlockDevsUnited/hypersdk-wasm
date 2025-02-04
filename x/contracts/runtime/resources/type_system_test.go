// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resources

import (
	"context"
	"testing"

	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
	"github.com/stretchr/testify/require"
)

func TestTypeSystem(t *testing.T) {
	r := require.New(t)
	ctx := context.Background()

	ts := NewTypeSystem()
	r.NotNil(ts)

	// Test type registration
	typ := core.ResourceType{
		Name:      "TestType",
		Abilities: []core.Ability{core.Key, core.Store},
	}

	err := ts.RegisterType(ctx, typ)
	r.NoError(err)

	// Test duplicate registration
	err = ts.RegisterType(ctx, typ)
	r.Error(err)
	r.Equal(ErrTypeAlreadyRegistered, err)

	// Test get type
	retrieved, err := ts.GetType("TestType")
	r.NoError(err)
	r.Equal(typ, retrieved)

	// Test get non-existent type
	_, err = ts.GetType("NonExistent")
	r.Error(err)
	r.Equal(ErrTypeNotRegistered, err)

	// Test list types
	types := ts.ListTypes()
	r.Len(types, 1)
	r.Equal(typ, types[0])
}

func TestTypeValidation(t *testing.T) {
	r := require.New(t)
	ctx := context.Background()

	ts := NewTypeSystem()

	// Test valid type
	validType := core.ResourceType{
		Name:      "ValidType",
		Abilities: []core.Ability{core.Key, core.Store},
	}

	err := ts.RegisterType(ctx, validType)
	r.NoError(err)

	// Test validation of registered type
	err = ts.ValidateType(validType)
	r.NoError(err)

	// Test validation of unregistered type
	unregisteredType := core.ResourceType{
		Name:      "UnregisteredType",
		Abilities: []core.Ability{core.Key, core.Store},
	}

	err = ts.ValidateType(unregisteredType)
	r.Error(err)
	r.Equal(ErrTypeNotRegistered, err)

	// Test validation of type with different abilities
	differentAbilities := core.ResourceType{
		Name:      "ValidType",
		Abilities: []core.Ability{core.Key},
	}

	err = ts.ValidateType(differentAbilities)
	r.Error(err)
}

func TestFieldValidation(t *testing.T) {
	r := require.New(t)
	ctx := context.Background()

	ts := NewTypeSystem()

	// Register a type that can be used as a field type
	compositeType := core.ResourceType{
		Name:      "CompositeType",
		Abilities: []core.Ability{core.Store},
	}

	err := ts.RegisterType(ctx, compositeType)
	r.NoError(err)

	testCases := []struct {
		name    string
		typ     core.ResourceType
		wantErr bool
	}{
		{
			name: "valid primitive fields",
			typ: core.ResourceType{
				Name:      "Test1",
				Abilities: []core.Ability{core.Key},
			},
			wantErr: false,
		},
		{
			name: "valid composite field",
			typ: core.ResourceType{
				Name:      "Test2",
				Abilities: []core.Ability{core.Key},
			},
			wantErr: false,
		},
		{
			name: "invalid field type",
			typ: core.ResourceType{
				Name:      "Test3",
				Abilities: []core.Ability{core.Key},
			},
			wantErr: true,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			err := ts.RegisterType(ctx, tc.typ)
			if tc.wantErr {
				r.Error(err)
			} else {
				r.NoError(err)
			}
		})
	}
}

func TestTypeSystem_ValidateResourceData(t *testing.T) {
	r := require.New(t)
	ctx := context.Background()
	ts := NewTypeSystem()

	// Register test type
	typ := core.ResourceType{
		Name:      "TestType",
		Abilities: []core.Ability{core.Key, core.Store},
	}

	err := ts.RegisterType(ctx, typ)
	r.NoError(err)

	testCases := []struct {
		name    string
		data    []byte
		wantErr bool
	}{
		{
			name:    "valid data",
			data:    []byte("valid data"),
			wantErr: false,
		},
		{
			name:    "missing field",
			data:    nil,
			wantErr: true,
		},
		{
			name:    "empty data",
			data:    []byte{},
			wantErr: true,
		},
	}

	for _, tc := range testCases {
		t.Run(tc.name, func(t *testing.T) {
			err := ts.ValidateResourceData(typ, tc.data)
			if tc.wantErr {
				r.Error(err)
				r.ErrorIs(err, ErrMissingField)
			} else {
				r.NoError(err)
			}
		})
	}
}

func TestTypeSystem_ValidateResourceData_UnregisteredType(t *testing.T) {
	r := require.New(t)
	ts := NewTypeSystem()

	typ := core.ResourceType{
		Name:      "UnregisteredType",
		Abilities: []core.Ability{core.Key, core.Store},
	}

	data := `{}`
	err := ts.ValidateResourceData(typ, []byte(data))
	r.ErrorIs(err, ErrTypeNotRegistered)
}
