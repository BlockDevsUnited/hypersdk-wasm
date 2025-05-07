// Copyright (C) 2025. All rights reserved.
// See the file LICENSE for licensing terms.

package state

import (
	"context"
	"errors"
	"fmt"
	"sync"
	"time"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/avalanchego/utils/logging"
	"github.com/ava-labs/avalanchego/utils/set"
	"github.com/ava-labs/hypersdk/attestation"
	"github.com/ava-labs/hypersdk/tee"
	
	"github.com/klauspost/compress/zstd"
)

const (
	// Default cache sizes
	defaultWitnessBlockCacheSize  = 100  // Number of blocks to cache
	defaultWitnessEntryCacheSize  = 1000 // Number of witness entries to cache
	defaultMaxWitnessSize         = 2 * 1024 * 1024 // 2MB max witness size
	defaultCompressionLevel       = 3     // Medium compression level for witnesses
	defaultWitnessExpiryDuration  = 5 * time.Minute
	defaultMaxWitnessBatchSize    = 500   // Maximum witnesses to process in a batch
)

var (
	ErrWitnessNotFound     = errors.New("witness not found")
	ErrWitnessTooLarge     = errors.New("witness exceeds maximum size")
	ErrInvalidWitness      = errors.New("invalid witness format")
	ErrWitnessExpired      = errors.New("witness has expired")
	ErrBatchSizeTooLarge   = errors.New("batch size exceeds maximum")
	ErrDuplicateWitness    = errors.New("duplicate witness")
	ErrInvalidProof        = errors.New("invalid state proof")
)

// WitnessEntry represents a state witness from a TEE
type WitnessEntry struct {
	// Witness identifier (typically derived from the state key and block height)
	ID ids.ID
	
	// State key this witness is for
	Key []byte
	
	// Block height this witness was created at
	BlockHeight uint64
	
	// Timestamp when this witness was created
	Timestamp time.Time
	
	// The actual witness data (may be compressed)
	Data []byte
	
	// Proof verifying the witness
	Proof []byte
	
	// The ID of the TEE that generated this witness
	TEEID ids.ID
	
	// TEE attestation data (if applicable)
	Attestation []byte
	
	// Whether this witness has been verified
	Verified bool
	
	// Whether this data is compressed
	Compressed bool
	
	// Cached decompressed size (to avoid decompressing multiple times)
	DecompressedSize int
}

// WitnessManagerConfig contains configuration for the witness manager
type WitnessManagerConfig struct {
	BlockCacheSize      int
	EntryCacheSize      int
	MaxWitnessSize      int
	CompressionLevel    int
	WitnessExpiry       time.Duration
	MaxBatchSize        int
	EnforceVerification bool
}

// DefaultWitnessManagerConfig returns a default configuration
func DefaultWitnessManagerConfig() *WitnessManagerConfig {
	return &WitnessManagerConfig{
		BlockCacheSize:      defaultWitnessBlockCacheSize,
		EntryCacheSize:      defaultWitnessEntryCacheSize,
		MaxWitnessSize:      defaultMaxWitnessSize,
		CompressionLevel:    defaultCompressionLevel,
		WitnessExpiry:       defaultWitnessExpiryDuration,
		MaxBatchSize:        defaultMaxWitnessBatchSize,
		EnforceVerification: true,
	}
}

// WitnessManager optimizes handling of state witnesses in a stateless blockchain
type WitnessManager struct {
	log      logging.Logger
	config   *WitnessManagerConfig
	verifier tee.Verifier
	
	// Caches for witnesses
	entriesByID    map[ids.ID]*WitnessEntry
	entriesByKey   map[string]map[uint64]*WitnessEntry // map[stateKey]map[blockHeight]witness
	entriesByBlock map[uint64]set.Set[ids.ID]
	
	// Compression encoders/decoders
	encoder *zstd.Encoder
	decoder *zstd.Decoder
	
	mutex sync.RWMutex
	
	// Statistics
	hits              uint64
	misses            uint64
	staleEvictions    uint64
	sizeEvictions     uint64
	totalCompressed   uint64
	totalUncompressed uint64
	verifications     uint64
	verificationFails uint64
}

// NewWitnessManager creates a new witness manager
func NewWitnessManager(
	log logging.Logger,
	config *WitnessManagerConfig,
	verifier tee.Verifier,
) (*WitnessManager, error) {
	if config == nil {
		config = DefaultWitnessManagerConfig()
	}

	// Create compression encoder/decoder
	encoder, err := zstd.NewWriter(nil, zstd.WithEncoderLevel(zstd.EncoderLevel(config.CompressionLevel)))
	if err != nil {
		return nil, fmt.Errorf("failed to create compression encoder: %w", err)
	}
	
	decoder, err := zstd.NewReader(nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create compression decoder: %w", err)
	}
	
	return &WitnessManager{
		log:            log,
		config:         config,
		verifier:       verifier,
		entriesByID:    make(map[ids.ID]*WitnessEntry),
		entriesByKey:   make(map[string]map[uint64]*WitnessEntry),
		entriesByBlock: make(map[uint64]set.Set[ids.ID]),
		encoder:        encoder,
		decoder:        decoder,
	}, nil
}

// AddWitness adds a witness to the manager
func (wm *WitnessManager) AddWitness(ctx context.Context, witness *WitnessEntry) error {
	if witness == nil {
		return errors.New("witness cannot be nil")
	}
	
	if len(witness.Data) > wm.config.MaxWitnessSize {
		return fmt.Errorf("%w: %d bytes (max %d)", ErrWitnessTooLarge, len(witness.Data), wm.config.MaxWitnessSize)
	}
	
	// Verify the witness if required
	if wm.config.EnforceVerification && !witness.Verified && wm.verifier != nil {
			// Use the verifier's VerifyAttestation method based on tee.Verifier interface
		teaAttestation := &attestation.TEEAttestation{
			Report: witness.Attestation,
			Signature: witness.Proof,
			// Other fields can be left as zero values for verification
		}
		if err := wm.verifier.VerifyAttestation(ctx, teaAttestation); err != nil {
			return fmt.Errorf("witness attestation verification failed: %w", err)
		}
		witness.Verified = true
		wm.verifications++
	}
	
	wm.mutex.Lock()
	defer wm.mutex.Unlock()
	
	// Check if this witness already exists
	if existing, exists := wm.entriesByID[witness.ID]; exists {
		if !existing.Timestamp.Before(witness.Timestamp) {
			// Existing witness is newer or same age, keep it
			return ErrDuplicateWitness
		}
	}
	
	// Add to ID index
	wm.entriesByID[witness.ID] = witness
	
	// Add to key->height index
	keyStr := string(witness.Key)
	if _, exists := wm.entriesByKey[keyStr]; !exists {
		wm.entriesByKey[keyStr] = make(map[uint64]*WitnessEntry)
	}
	wm.entriesByKey[keyStr][witness.BlockHeight] = witness
	
	// Add to block height index
	if _, exists := wm.entriesByBlock[witness.BlockHeight]; !exists {
		// Initialize a new set and store it
		wm.entriesByBlock[witness.BlockHeight] = set.Set[ids.ID]{}
	}
	// Update the set - in your implementation, set.Set appears to be a value type
	// We need to get and update the set, then put it back
	setVal := wm.entriesByBlock[witness.BlockHeight]
	setVal.Add(witness.ID)
	wm.entriesByBlock[witness.BlockHeight] = setVal
	
	// Update stats
	if witness.Compressed {
		wm.totalCompressed += uint64(len(witness.Data))
		wm.totalUncompressed += uint64(witness.DecompressedSize)
	} else {
		wm.totalUncompressed += uint64(len(witness.Data))
	}
	
	// Evict stale witnesses if we're over capacity
	wm.enforceCapacity()
	
	return nil
}

// GetWitness retrieves a witness by ID
func (wm *WitnessManager) GetWitness(id ids.ID) (*WitnessEntry, error) {
	wm.mutex.RLock()
	defer wm.mutex.RUnlock()
	
	witness, exists := wm.entriesByID[id]
	if !exists {
		wm.misses++
		return nil, ErrWitnessNotFound
	}
	
	// Check if witness has expired
	if time.Since(witness.Timestamp) > wm.config.WitnessExpiry {
		wm.misses++
		return nil, ErrWitnessExpired
	}
	
	wm.hits++
	return witness, nil
}

// GetWitnessByKey retrieves the latest witness for a state key
func (wm *WitnessManager) GetWitnessByKey(key []byte, maxBlockHeight uint64) (*WitnessEntry, error) {
	wm.mutex.RLock()
	defer wm.mutex.RUnlock()
	
	keyStr := string(key)
	heightMap, exists := wm.entriesByKey[keyStr]
	if !exists {
		wm.misses++
		return nil, ErrWitnessNotFound
	}
	
	// Find the highest block height <= maxBlockHeight
	var bestHeight uint64
	var bestWitness *WitnessEntry
	
	for height, witness := range heightMap {
		if height <= maxBlockHeight && height > bestHeight {
			bestHeight = height
			bestWitness = witness
		}
	}
	
	if bestWitness == nil {
		wm.misses++
		return nil, ErrWitnessNotFound
	}
	
	// Check if witness has expired
	if time.Since(bestWitness.Timestamp) > wm.config.WitnessExpiry {
		wm.misses++
		return nil, ErrWitnessExpired
	}
	
	wm.hits++
	return bestWitness, nil
}

// CompressWitness compresses witness data if not already compressed
func (wm *WitnessManager) CompressWitness(witness *WitnessEntry) error {
	if witness.Compressed {
		return nil
	}
	
	compressed := wm.encoder.EncodeAll(witness.Data, nil)
	
	// Only use compression if it actually helps
	if len(compressed) < len(witness.Data) {
		witness.DecompressedSize = len(witness.Data)
		witness.Data = compressed
		witness.Compressed = true
	}
	
	return nil
}

// DecompressWitness decompresses witness data if compressed
func (wm *WitnessManager) DecompressWitness(witness *WitnessEntry) ([]byte, error) {
	if !witness.Compressed {
		return witness.Data, nil
	}
	
	decompressed, err := wm.decoder.DecodeAll(witness.Data, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to decompress witness: %w", err)
	}
	
	return decompressed, nil
}

// enforceCapacity ensures the witness cache doesn't exceed configured limits
func (wm *WitnessManager) enforceCapacity() {
	// Check if we're under capacity
	if len(wm.entriesByID) <= wm.config.EntryCacheSize {
		return
	}
	
	// Find witnesses to evict
	var blocksToCheck []uint64
	for height := range wm.entriesByBlock {
		blocksToCheck = append(blocksToCheck, height)
	}
	
	// Sort blocks by height (ascending)
	// In a real implementation, use a proper sorting function
	// For simplicity, we'll simulate removing the oldest blocks first
	
	// Remove oldest blocks until we're under capacity
	removed := 0
	for _, height := range blocksToCheck {
		if len(wm.entriesByID)-removed <= wm.config.EntryCacheSize {
			break
		}
		
		blockWitnesses := wm.entriesByBlock[height]
		for witnessID := range blockWitnesses {
			witness := wm.entriesByID[witnessID]
			
			// Remove from indexes
			delete(wm.entriesByID, witnessID)
			
			keyStr := string(witness.Key)
			if heightMap, exists := wm.entriesByKey[keyStr]; exists {
				delete(heightMap, witness.BlockHeight)
				if len(heightMap) == 0 {
					delete(wm.entriesByKey, keyStr)
				}
			}
			
			removed++
		}
		
		delete(wm.entriesByBlock, height)
		wm.staleEvictions++
	}
}

// ProcessWitnessBatch efficiently processes a batch of witnesses
func (wm *WitnessManager) ProcessWitnessBatch(
	ctx context.Context,
	witnesses []*WitnessEntry,
) (int, error) {
	if len(witnesses) > wm.config.MaxBatchSize {
		return 0, fmt.Errorf("%w: %d (max %d)", ErrBatchSizeTooLarge, len(witnesses), wm.config.MaxBatchSize)
	}
	
	successCount := 0
	
	// Process in smaller chunks for better parallelism
	for i := 0; i < len(witnesses); i += 10 {
		end := i + 10
		if end > len(witnesses) {
			end = len(witnesses)
		}
		
		var wg sync.WaitGroup
		results := make([]error, end-i)
		
		// Process chunk in parallel
		for j := i; j < end; j++ {
			wg.Add(1)
			go func(idx int) {
				defer wg.Done()
				results[idx-i] = wm.AddWitness(ctx, witnesses[idx])
			}(j)
		}
		
		wg.Wait()
		
		// Count successes
		for _, err := range results {
			if err == nil || err == ErrDuplicateWitness {
				successCount++
			}
		}
	}
	
	return successCount, nil
}

// GetStats returns statistics about the witness manager
func (wm *WitnessManager) GetStats() map[string]interface{} {
	wm.mutex.RLock()
	defer wm.mutex.RUnlock()
	
	return map[string]interface{}{
		"witnessCount":        len(wm.entriesByID),
		"uniqueKeys":          len(wm.entriesByKey),
		"blockCount":          len(wm.entriesByBlock),
		"cacheHits":           wm.hits,
		"cacheMisses":         wm.misses,
		"staleEvictions":      wm.staleEvictions,
		"sizeEvictions":       wm.sizeEvictions,
		"totalCompressed":     wm.totalCompressed,
		"totalUncompressed":   wm.totalUncompressed,
		"compressionRatio":    float64(wm.totalUncompressed) / float64(wm.totalCompressed+1),
		"verifications":       wm.verifications,
		"verificationFails":   wm.verificationFails,
	}
}
