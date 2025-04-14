// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resources

import (
	"bytes"
	"encoding/binary"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
)

var _ core.Resource = &BaseResource{}

// BaseResource implements the core.Resource interface
type BaseResource struct {
	typ       core.ResourceType
	id        core.ResourceID
	ownerAddr codec.Address
	data      []byte
	moved     bool
}

// NewBaseResource creates a new base resource
func NewBaseResource(id core.ResourceID, typ core.ResourceType, owner codec.Address, data []byte) *BaseResource {
	// Create a copy of the ID to ensure it's not modified externally
	var newID core.ResourceID
	copy(newID[:], id[:])

	// Create a copy of the data to ensure it's not modified externally
	dataCopy := make([]byte, len(data))
	copy(dataCopy, data)

	return &BaseResource{
		typ:       typ,
		id:        newID,
		ownerAddr: owner,
		data:      dataCopy,
	}
}

// Type returns the resource type
func (r *BaseResource) Type() core.ResourceType {
	return r.typ
}

// ID returns the resource ID
func (r *BaseResource) ID() core.ResourceID {
	return r.id
}

// Owner returns the current owner of the resource
func (r *BaseResource) Owner() codec.Address {
	return r.ownerAddr
}

// IsMoved returns whether the resource has been moved
func (r *BaseResource) IsMoved() bool {
	return r.moved
}

// MarkMoved marks the resource as moved
func (r *BaseResource) MarkMoved() {
	r.moved = true
}

// Data returns the resource data
func (r *BaseResource) Data() []byte {
	return r.data
}

// Marshal serializes the resource to bytes
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

// Unmarshal deserializes the resource from bytes
func (r *BaseResource) Unmarshal(data []byte) error {
	if len(data) < 73 { // minimum size: 32 (id) + 4 (name length) + 4 (abilities length) + 32 (owner) + 1 (moved flag)
		return core.ErrInvalidResourceData
	}

	offset := 0

	// Read resource ID
	copy(r.id[:], data[offset:offset+32])
	offset += 32

	// Read name
	nameLen := binary.BigEndian.Uint32(data[offset:])
	offset += 4
	if offset+int(nameLen) > len(data) {
		return core.ErrInvalidResourceData
	}
	r.typ.Name = string(data[offset:offset+int(nameLen)])
	offset += int(nameLen)

	// Read abilities
	if offset+4 > len(data) {
		return core.ErrInvalidResourceData
	}
	abilitiesLen := binary.BigEndian.Uint32(data[offset:])
	offset += 4
	r.typ.Abilities = make([]core.Ability, abilitiesLen)
	for i := uint32(0); i < abilitiesLen; i++ {
		if offset+8 > len(data) {
			return core.ErrInvalidResourceData
		}
		// Trim trailing zeros
		abilityBytes := bytes.TrimRight(data[offset:offset+8], "\x00")
		r.typ.Abilities[i] = core.Ability(abilityBytes)
		offset += 8
	}

	// Read owner address
	if offset+32 > len(data) {
		return core.ErrInvalidResourceData
	}
	copy(r.ownerAddr[:], data[offset:offset+32])
	offset += 32

	// Read data
	if offset+4 > len(data) {
		return core.ErrInvalidResourceData
	}
	dataLen := binary.BigEndian.Uint32(data[offset:])
	offset += 4
	if offset+int(dataLen) > len(data)-1 { // -1 for moved flag
		return core.ErrInvalidResourceData
	}
	r.data = make([]byte, dataLen)
	copy(r.data, data[offset:offset+int(dataLen)])
	offset += int(dataLen)

	// Read moved flag
	r.moved = data[offset] == 1

	return nil
}
