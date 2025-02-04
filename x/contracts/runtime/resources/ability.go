// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resources

import (
	"errors"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
)

var (
	ErrInvalidAbility = errors.New("invalid ability")
)

// AbilitySet represents a set of abilities that a resource type has
type AbilitySet struct {
	abilities map[core.Ability]bool
}

// NewAbilitySet creates a new ability set with the given abilities
func NewAbilitySet(abilities ...core.Ability) *AbilitySet {
	set := &AbilitySet{
		abilities: make(map[core.Ability]bool),
	}
	for _, ability := range abilities {
		set.abilities[ability] = true
	}
	return set
}

// Has returns true if the set has the given ability
func (s *AbilitySet) Has(ability core.Ability) bool {
	return s.abilities[ability]
}

// Add adds an ability to the set
func (s *AbilitySet) Add(ability core.Ability) {
	s.abilities[ability] = true
}

// Remove removes an ability from the set
func (s *AbilitySet) Remove(ability core.Ability) {
	delete(s.abilities, ability)
}

// List returns a slice of all abilities in the set
func (s *AbilitySet) List() []core.Ability {
	abilities := make([]core.Ability, 0, len(s.abilities))
	for ability := range s.abilities {
		abilities = append(abilities, ability)
	}
	return abilities
}

// ValidateAbilities checks if all abilities in the set are valid
func ValidateAbilities(abilities []core.Ability) error {
	for _, ability := range abilities {
		switch ability {
		case core.Key, core.Store, core.Drop:
			// Valid abilities
		default:
			return ErrInvalidAbility
		}
	}
	return nil
}

// RequiredAbilities returns the abilities that are required for a given operation
func RequiredAbilities(operation string) []core.Ability {
	switch operation {
	case "store":
		return []core.Ability{core.Store}
	case "drop":
		return []core.Ability{core.Drop}
	case "move":
		return []core.Ability{core.Key}
	default:
		return nil
	}
}
