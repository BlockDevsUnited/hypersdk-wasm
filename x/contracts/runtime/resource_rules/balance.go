// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resource_rules

import (
	"encoding/binary"
	"errors"

	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
)

var (
	ErrNegativeBalance = errors.New("negative balance")
	ErrInvalidData     = errors.New("invalid balance data")
)

// ValidateBalanceNotNegative ensures a balance resource has a non-negative amount
func ValidateBalanceNotNegative(resource core.Resource) error {
	data := resource.Data()
	if len(data) < 8 {
		return ErrInvalidData
	}

	amount := binary.BigEndian.Uint64(data)
	if amount == 0 {
		return ErrNegativeBalance
	}

	return nil
}
