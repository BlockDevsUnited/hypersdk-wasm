package regional

import (
	"fmt"
	"time"

	"github.com/ava-labs/avalanchego/ids"
	"go.uber.org/zap"
)

// ConflictDetectionConfig holds configuration parameters for conflict detection
type ConflictDetectionConfig struct {
	// Maximum allowed timestamp difference in milliseconds
	MaxTimestampDiffMs int64
	
	// Whether local transactions take precedence over cross-region in conflicts
	LocalPriority bool

	// Whether to enable audit trail for conflict resolutions
	EnableAuditTrail bool
}

// DefaultConflictDetectionConfig returns the default configuration
func DefaultConflictDetectionConfig() *ConflictDetectionConfig {
	return &ConflictDetectionConfig{
		MaxTimestampDiffMs: 100, // Aligns with "100ms and regulated" value proposition
		LocalPriority:      false,
		EnableAuditTrail:   true,
	}
}

// ConflictType represents the type of conflict detected
type ConflictType int

const (
	NoConflict ConflictType = iota
	KeyConflict
	TimestampConflict
	RegulatoryConflict
)

// String returns a string representation of the conflict type
func (ct ConflictType) String() string {
	switch ct {
	case NoConflict:
		return "NoConflict"
	case KeyConflict:
		return "KeyConflict"
	case TimestampConflict:
		return "TimestampConflict"
	case RegulatoryConflict:
		return "RegulatoryConflict"
	default:
		return fmt.Sprintf("Unknown(%d)", ct)
	}
}

// ConflictResolution represents how a conflict was resolved
type ConflictResolution int

const (
	NoResolutionNeeded ConflictResolution = iota
	LocalWins
	CrossRegionWins
	Rejected
)

// String returns a string representation of the conflict resolution
func (cr ConflictResolution) String() string {
	switch cr {
	case NoResolutionNeeded:
		return "NoResolutionNeeded"
	case LocalWins:
		return "LocalWins"
	case CrossRegionWins:
		return "CrossRegionWins"
	case Rejected:
		return "Rejected"
	default:
		return fmt.Sprintf("Unknown(%d)", cr)
	}
}

// ConflictMetadata stores information about a detected conflict
type ConflictMetadata struct {
	Type            ConflictType
	Key             string
	LocalTimestamp  time.Time
	RemoteTimestamp time.Time
	Resolution      ConflictResolution
	LocalTxID       ids.ID
	RemoteTxID      ids.ID
}

// ConflictDetector provides methods to detect and resolve state conflicts
type ConflictDetector struct {
	config *ConflictDetectionConfig
	log    *zap.Logger
	
	// Audit trail of conflict resolutions if enabled
	auditTrail []ConflictMetadata
}

// NewConflictDetector creates a new conflict detector with the specified config
func NewConflictDetector(config *ConflictDetectionConfig, log *zap.Logger) *ConflictDetector {
	if config == nil {
		config = DefaultConflictDetectionConfig()
	}
	
	return &ConflictDetector{
		config: config,
		log:    log,
		auditTrail: make([]ConflictMetadata, 0),
	}
}

// DetectConflicts checks for conflicts between pending local state updates and incoming cross-region operations
// Returns a map of keys with their conflict status and metadata
func (cd *ConflictDetector) DetectConflicts(localUpdates map[string][]byte, localTimestamp time.Time, localTxID ids.ID, 
	remoteUpdates map[string][]byte, remoteTimestamp time.Time, remoteTxID ids.ID) map[string]ConflictMetadata {
	
	conflicts := make(map[string]ConflictMetadata)
	
	// Check for key conflicts
	for key := range remoteUpdates {
		if _, exists := localUpdates[key]; exists {
			// We have a potential key conflict
			timestampDiff := localTimestamp.Sub(remoteTimestamp).Milliseconds()
			
			var conflictType ConflictType
			if abs(timestampDiff) > cd.config.MaxTimestampDiffMs {
				conflictType = TimestampConflict
			} else {
				conflictType = KeyConflict
			}
			
			metadata := ConflictMetadata{
				Type:            conflictType,
				Key:             key,
				LocalTimestamp:  localTimestamp,
				RemoteTimestamp: remoteTimestamp,
				LocalTxID:       localTxID,
				RemoteTxID:      remoteTxID,
			}
			
			conflicts[key] = metadata
			
			cd.log.Debug("State conflict detected",
				zap.String("key", key),
				zap.String("conflictType", conflictType.String()),
				zap.Int64("timestampDiffMs", timestampDiff),
			)
		}
	}
	
	return conflicts
}

// ResolveConflicts applies resolution strategies to detected conflicts
func (cd *ConflictDetector) ResolveConflicts(conflicts map[string]ConflictMetadata) map[string]ConflictResolution {
	resolutions := make(map[string]ConflictResolution)
	
	for key, metadata := range conflicts {
		var resolution ConflictResolution
		
		switch metadata.Type {
		case KeyConflict:
			if cd.config.LocalPriority {
				resolution = LocalWins
			} else {
				// Default to remote taking precedence for cross-region operations
				resolution = CrossRegionWins
			}
			
		case TimestampConflict:
			if cd.config.LocalPriority {
				// If LocalPriority is true, local operations win regardless of timestamp
				resolution = LocalWins
			} else {
				// Otherwise, newer timestamp wins
				if metadata.LocalTimestamp.After(metadata.RemoteTimestamp) {
					resolution = LocalWins
				} else {
					resolution = CrossRegionWins
				}
			}
			
		case RegulatoryConflict:
			// Regulatory conflicts always result in rejection
			resolution = Rejected
			
		default:
			resolution = NoResolutionNeeded
		}
		
		resolutions[key] = resolution
		
		// Update conflict metadata with resolution
		metadata.Resolution = resolution
		
		// Store in audit trail if enabled
		if cd.config.EnableAuditTrail {
			cd.auditTrail = append(cd.auditTrail, metadata)
		}
		
		cd.log.Info("Conflict resolution",
			zap.String("key", key),
			zap.String("conflictType", metadata.Type.String()),
			zap.String("resolution", resolution.String()),
		)
	}
	
	return resolutions
}

// GetAuditTrail returns the conflict resolution audit trail
func (cd *ConflictDetector) GetAuditTrail() []ConflictMetadata {
	return cd.auditTrail
}

// ClearAuditTrail clears the stored audit trail
func (cd *ConflictDetector) ClearAuditTrail() {
	cd.auditTrail = make([]ConflictMetadata, 0)
}

// abs returns the absolute value of an int64
func abs(x int64) int64 {
	if x < 0 {
		return -x
	}
	return x
}
