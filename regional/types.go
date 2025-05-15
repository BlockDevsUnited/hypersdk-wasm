// See the file LICENSE for licensing terms.

package regional

import (
	"time"

	"github.com/ava-labs/avalanchego/ids"
)

// BatchOperation represents a batched transaction operation across regions
type BatchOperation struct {
	TxID          ids.ID
	PrimaryRegion string
	OtherRegions  []string
	Timestamp     int64
	Payload       []byte
}

// BatchResult represents the result of a batch operation for a region
type BatchResult struct {
	Success bool
	Error   string
	Region  string
	Metrics map[string]interface{}
}

// Config defines configuration for the regional manager
type Config struct {
	// PrimaryRegion is the default region for this node
	PrimaryRegion string
	
	// Regions is the list of all supported regions
	Regions []string
	
	// DefaultRegion is used when no region is specified
	DefaultRegion string
	
	// TEEAttestationRequired determines if TEE attestation is required
	TEEAttestationRequired bool
	
	// AttestationTypes defines the supported attestation types (e.g., sgx, sev)
	AttestationTypes []string
	
	// TimestampVerificationRequired determines if timestamp verification is required
	TimestampVerificationRequired bool
	
	// MaxFutureTimestampWindow is the maximum allowed time in the future for timestamps
	MaxFutureTimestampWindow time.Duration
	
	// MaxPastTimestampWindow is the maximum allowed time in the past for timestamps
	MaxPastTimestampWindow time.Duration
	
	// MeshBatchSize is the batch size for mesh network operations
	MeshBatchSize int
}

// TEEMeshVerifier has been moved to manager.go

// RegionalStatus represents the status of a region
type RegionalStatus struct {
	// IsActive indicates if the region is currently active
	IsActive bool
	
	// LastAttestation is the timestamp of the last successful attestation
	LastAttestation time.Time
	
	// Performance metrics
	AverageResponseTime time.Duration
	SuccessRate         float64
	ThroughputTPS       int
}
