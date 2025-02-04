// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resources

import "errors"

var (
	ErrInvalidTransfer = errors.New("invalid resource transfer")
	ErrResourceAlreadyExists = errors.New("resource already exists")
)
