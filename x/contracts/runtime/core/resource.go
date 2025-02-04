// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package core

import (
	"bytes"
	"encoding/binary"
	"github.com/ava-labs/hypersdk/codec"
	"errors"
)

// ResourceID uniquely identifies a resource
type ResourceID [32]byte

// Ability represents a capability of a resource
type Ability string

const (
	Key   Ability = "key"
	Store Ability = "store"
	Drop  Ability = "drop"
)

// ResourceType defines the type of a resource
type ResourceType struct {
	Name      string
	Abilities []Ability
}

// Resource represents a resource in the system
type Resource interface {
	Type() ResourceType
	ID() ResourceID
	Owner() codec.Address
	IsMoved() bool
	MarkMoved()
	Data() []byte
	Marshal() ([]byte, error)
	Unmarshal([]byte) error
}

// BaseResource is a basic implementation of the Resource interface
type BaseResource struct {
	typ       ResourceType
	id        ResourceID
	ownerAddr codec.Address
	data      []byte
	moved     bool
}

// NewBaseResource creates a new base resource
func NewBaseResource(id ResourceID, typ ResourceType, owner codec.Address, data []byte) *BaseResource {
	return &BaseResource{
		id:        id,
		typ:       typ,
		ownerAddr: owner,
		data:      data,
		moved:     false,
	}
}

func (r *BaseResource) Type() ResourceType     { return r.typ }
func (r *BaseResource) ID() ResourceID         { return r.id }
func (r *BaseResource) Owner() codec.Address   { return r.ownerAddr }
func (r *BaseResource) IsMoved() bool          { return r.moved }
func (r *BaseResource) MarkMoved()             { r.moved = true }
func (r *BaseResource) Data() []byte           { return r.data }

func (r *BaseResource) Marshal() ([]byte, error) {
	// Calculate total size
	nameBytes := []byte(r.typ.Name)
	totalSize := 32 + // resource ID
		4 + len(nameBytes) + // name length + name
		4 + len(r.typ.Abilities)*8 + // abilities length + abilities
		32 + // owner address
		4 + len(r.data) + // data length + data
		1 // moved flag

	result := make([]byte, totalSize)
	offset := 0

	// Write resource ID
	copy(result[offset:], r.id[:])
	offset += 32

	// Write name
	binary.BigEndian.PutUint32(result[offset:], uint32(len(nameBytes)))
	offset += 4
	copy(result[offset:], nameBytes)
	offset += len(nameBytes)

	// Write abilities
	binary.BigEndian.PutUint32(result[offset:], uint32(len(r.typ.Abilities)))
	offset += 4
	for _, ability := range r.typ.Abilities {
		abilityBytes := []byte(ability)
		if len(abilityBytes) > 8 {
			abilityBytes = abilityBytes[:8]
		} else {
			// Pad with zeros
			padded := make([]byte, 8)
			copy(padded, abilityBytes)
			abilityBytes = padded
		}
		copy(result[offset:], abilityBytes)
		offset += 8
	}

	// Write owner address
	copy(result[offset:], r.ownerAddr[:])
	offset += 32

	// Write data
	binary.BigEndian.PutUint32(result[offset:], uint32(len(r.data)))
	offset += 4
	copy(result[offset:], r.data)
	offset += len(r.data)

	// Write moved flag
	if r.moved {
		result[offset] = 1
	}

	return result, nil
}

func (r *BaseResource) Unmarshal(data []byte) error {
	if len(data) < 73 { // minimum size: 32 (id) + 4 (name length) + 4 (abilities length) + 32 (owner) + 1 (moved flag)
		return ErrInvalidResourceData
	}

	offset := 0

	// Read resource ID
	copy(r.id[:], data[offset:offset+32])
	offset += 32

	// Read name
	nameLen := binary.BigEndian.Uint32(data[offset:])
	offset += 4
	if offset+int(nameLen) > len(data) {
		return ErrInvalidResourceData
	}
	r.typ.Name = string(data[offset:offset+int(nameLen)])
	offset += int(nameLen)

	// Read abilities
	if offset+4 > len(data) {
		return ErrInvalidResourceData
	}
	abilitiesLen := binary.BigEndian.Uint32(data[offset:])
	offset += 4
	r.typ.Abilities = make([]Ability, abilitiesLen)
	for i := uint32(0); i < abilitiesLen; i++ {
		if offset+8 > len(data) {
			return ErrInvalidResourceData
		}
		// Trim trailing zeros
		abilityBytes := bytes.TrimRight(data[offset:offset+8], "\x00")
		r.typ.Abilities[i] = Ability(abilityBytes)
		offset += 8
	}

	// Read owner address
	if offset+32 > len(data) {
		return ErrInvalidResourceData
	}
	copy(r.ownerAddr[:], data[offset:offset+32])
	offset += 32

	// Read data
	if offset+4 > len(data) {
		return ErrInvalidResourceData
	}
	dataLen := binary.BigEndian.Uint32(data[offset:])
	offset += 4
	if offset+int(dataLen) > len(data)-1 { // -1 for moved flag
		return ErrInvalidResourceData
	}
	r.data = make([]byte, dataLen)
	copy(r.data, data[offset:offset+int(dataLen)])
	offset += int(dataLen)

	// Read moved flag
	r.moved = data[offset] == 1

	return nil
}

// ResourceValidator validates resource operations
type ResourceValidator interface {
	// Type validation
	ValidateResourceType(typ ResourceType) error
	
	// Operation validation
	ValidateMove(resource Resource, from, to codec.Address) error
	ValidateUpdate(resource Resource, newValue []byte) error
}

// ValidateAbilities checks if all abilities in the set are valid
func ValidateAbilities(abilities []Ability) error {
	for _, ability := range abilities {
		switch ability {
		case Key, Store, Drop:
			continue
		default:
			return ErrInvalidAbility
		}
	}
	return nil
}

// Error definitions
var (
	ErrInvalidAbility       = errors.New("invalid ability")
	ErrInvalidResourceData  = errors.New("invalid resource data")
)
