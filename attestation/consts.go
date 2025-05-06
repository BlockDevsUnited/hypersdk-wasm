// Copyright (C) 2025. All rights reserved.
// See the file LICENSE for licensing terms.

package attestation

// Constants for attestation validation
const (
	// MaxTimeWindow is the maximum allowed drift between attestation timestamp and current time (in ms)
	MaxTimeWindow int64 = 60 * 1000 // 60 seconds
)
