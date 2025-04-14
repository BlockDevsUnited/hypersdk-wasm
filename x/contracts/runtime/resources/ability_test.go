// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resources

import (
	"testing"

	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
	"github.com/stretchr/testify/require"
)

func TestAbilitySet(t *testing.T) {
	r := require.New(t)

	// Test creation
	abilities := []core.Ability{core.Key, core.Store}
	set := NewAbilitySet(abilities...)

	// Test Has
	r.True(set.Has(core.Key))
	r.True(set.Has(core.Store))
	r.False(set.Has(core.Drop))

	// Test Add
	set.Add(core.Drop)
	r.True(set.Has(core.Drop))

	// Test Remove
	set.Remove(core.Store)
	r.False(set.Has(core.Store))

	// Test List
	list := set.List()
	r.Len(list, 2)
	r.Contains(list, core.Key)
	r.Contains(list, core.Drop)
}

func TestValidateAbilities(t *testing.T) {
	r := require.New(t)

	// Test valid abilities
	validAbilities := []core.Ability{core.Key, core.Store, core.Drop}
	err := ValidateAbilities(validAbilities)
	r.NoError(err)

	// Test invalid ability
	invalidAbilities := []core.Ability{core.Key, "invalid"}
	err = ValidateAbilities(invalidAbilities)
	r.Error(err)
	r.Equal(ErrInvalidAbility, err)
}

func TestRequiredAbilities(t *testing.T) {
	r := require.New(t)

	// Test store operation
	storeAbilities := RequiredAbilities("store")
	r.Len(storeAbilities, 1)
	r.Equal(core.Store, storeAbilities[0])

	// Test drop operation
	dropAbilities := RequiredAbilities("drop")
	r.Len(dropAbilities, 1)
	r.Equal(core.Drop, dropAbilities[0])

	// Test move operation
	moveAbilities := RequiredAbilities("move")
	r.Len(moveAbilities, 1)
	r.Equal(core.Key, moveAbilities[0])

	// Test invalid operation
	invalidAbilities := RequiredAbilities("invalid")
	r.Nil(invalidAbilities)
}
