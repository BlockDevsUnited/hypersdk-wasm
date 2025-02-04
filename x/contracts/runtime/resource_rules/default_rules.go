// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package resource_rules

import (
	"github.com/ava-labs/hypersdk/x/contracts/runtime/core"
)

// DefaultResourceOperationRules defines the default rules for resource operations
var DefaultResourceOperationRules = []ResourceOperationRule{
	{
		Type: core.ResourceType{
			Name:      "Balance",
			Abilities: []core.Ability{core.Key, core.Store},
		},
		CustomValidators: []func(core.Resource) error{
			ValidateBalanceNotNegative,
		},
	},
}
