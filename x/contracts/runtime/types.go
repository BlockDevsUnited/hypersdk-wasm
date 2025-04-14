// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

// RawBytesArrayInput represents raw bytes passed from WebAssembly to the host
type RawBytesArrayInput struct {
	Bytes []byte
}

// void represents an empty return type
type void struct{}
