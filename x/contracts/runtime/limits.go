// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import "github.com/ava-labs/avalanchego/utils/units"

// ResourceLimits defines constraints for WebAssembly contracts
type ResourceLimits struct {
	// Maximum size of contract bytecode in bytes
	MaxContractSize uint32

	// Maximum number of functions in a module
	MaxFunctions uint32

	// Maximum number of imports in a module
	MaxImports uint32

	// Maximum number of exports in a module
	MaxExports uint32

	// Maximum number of globals in a module
	MaxGlobals uint32

	// Maximum initial memory pages (64KB per page)
	MaxInitialMemoryPages uint32

	// Maximum memory pages after growth
	MaxMemoryPages uint32

	// Maximum table size
	MaxTableSize uint32
}

// DefaultResourceLimits returns resource limits with safe default values
func DefaultResourceLimits() ResourceLimits {
	return ResourceLimits{
		MaxContractSize:       1 * units.MiB,  // 1MB
		MaxFunctions:          1000,           // 1K functions
		MaxImports:           100,            // 100 imports
		MaxExports:           100,            // 100 exports
		MaxGlobals:           100,            // 100 globals
		MaxInitialMemoryPages: 4,              // 256KB (64KB * 4)
		MaxMemoryPages:        16,             // 1MB (64KB * 16)
		MaxTableSize:          10000,          // 10K table entries
	}
}
