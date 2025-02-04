// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resources

import (
	"encoding/binary"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
)

// BalanceResource represents a balance that can be transferred between addresses
type BalanceResource struct {
	resourceType core.ResourceType
	id          core.ResourceID
	owner       codec.Address
	amount      uint64
	moved       bool
}

// NewBalanceResource creates a new balance resource
func NewBalanceResource(id core.ResourceID, owner codec.Address, amount uint64) *BalanceResource {
	return &BalanceResource{
		resourceType: core.ResourceType{
			Name:      "Balance",
			Abilities: []core.Ability{core.Key, core.Store},
		},
		id:     id,
		owner:  owner,
		amount: amount,
	}
}

// Type returns the resource type
func (b *BalanceResource) Type() core.ResourceType {
	return b.resourceType
}

// ID returns the resource ID
func (b *BalanceResource) ID() core.ResourceID {
	return b.id
}

// Owner returns the current owner of the resource
func (b *BalanceResource) Owner() codec.Address {
	return b.owner
}

// Amount returns the current balance amount
func (b *BalanceResource) Amount() uint64 {
	return b.amount
}

// IsMoved returns whether the resource has been moved
func (b *BalanceResource) IsMoved() bool {
	return b.moved
}

// MarkMoved marks the resource as moved
func (b *BalanceResource) MarkMoved() {
	b.moved = true
}

// Data returns the resource data as bytes
func (b *BalanceResource) Data() []byte {
	data := make([]byte, 8) // uint64 size
	binary.BigEndian.PutUint64(data, b.amount)
	return data
}

// Marshal serializes the resource to bytes
func (b *BalanceResource) Marshal() ([]byte, error) {
	return b.Data(), nil
}

// Unmarshal deserializes the resource from bytes
func (b *BalanceResource) Unmarshal(data []byte) error {
	if len(data) < 8 {
		return core.ErrInvalidResourceData
	}
	b.amount = binary.BigEndian.Uint64(data)
	return nil
}
