// Copyright (C) Aristo Group, Inc. All rights reserved.
package chain

import (
	"bytes"
	"context"
	"errors"
	"fmt"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/avalanchego/utils/maybe"
	"github.com/ava-labs/avalanchego/x/merkledb"
	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/consts"
	"github.com/ava-labs/hypersdk/state"
)

const (
	// CommitmentKeyPrefix is the prefix for storing polynomial commitments in the state
	CommitmentKeyPrefix = "polynomial-commitment-"
)

var (
	// ErrInvalidCommitment is returned when a commitment is invalid
	ErrInvalidCommitment = errors.New("invalid polynomial commitment")
	
	// ErrInvalidProof is returned when a proof is invalid
	ErrInvalidProof = errors.New("invalid polynomial proof")
	
	// ErrMismatchedRoot is returned when a proof doesn't match the commitment root
	ErrMismatchedRoot = errors.New("proof root does not match commitment root")
)

// PolynomialCommitment represents a commitment to a polynomial
type PolynomialCommitment struct {
	// Root is the Merkle root of the polynomial
	Root ids.ID
	
	// Coefficients contains the committed polynomial coefficients
	Coefficients []byte
	
	// Timestamp is the time at which the commitment was created
	Timestamp uint64
}

// MarshalLen returns the length of the encoding of p.
func (p *PolynomialCommitment) MarshalLen() int {
	return ids.IDLen + // Root
		codec.BytesLen(p.Coefficients) + // Coefficients
		consts.Uint64Len // Timestamp
}

// Marshal marshals a polynomial commitment to bytes
func (p *PolynomialCommitment) Marshal(dst []byte) ([]byte, error) {
	packer := codec.NewWriter(p.MarshalLen(), consts.MaxInt)
	
	// Pack the ID
	packer.PackFixedBytes(p.Root[:])
	
	// Pack the coefficients
	packer.PackBytes(p.Coefficients)
	
	// Pack the timestamp
	packer.PackUint64(p.Timestamp)

	return packer.Bytes(), packer.Err()
}

// KeyEntry is a key-value entry in a polynomial proof
type KeyEntry struct {
	Key   []byte
	Value []byte
}

// PolynomialProof represents a proof of a polynomial evaluation
type PolynomialProof struct {
	// Root is the Merkle root the proof is for
	Root ids.ID
	
	// Entries are the key-value pairs included in the proof
	Entries []KeyEntry
	
	// Timestamp is the time at which the proof was created
	Timestamp uint64
}

// MarshalLen returns the length of the encoding of p.
func (p *PolynomialProof) MarshalLen() int {
	size := ids.IDLen + // Root
		consts.Uint64Len + // Timestamp
		consts.Uint64Len // Number of entries

	for _, entry := range p.Entries {
		size += codec.BytesLen(entry.Key) + // Key
			codec.BytesLen(entry.Value) // Value
	}
	
	return size
}

// Marshal marshals a polynomial proof to bytes
func (p *PolynomialProof) Marshal(dst []byte) ([]byte, error) {
	packer := codec.NewWriter(p.MarshalLen(), consts.MaxInt)
	
	// Pack the ID
	packer.PackFixedBytes(p.Root[:])
	
	// Pack the timestamp
	packer.PackUint64(p.Timestamp)
	
	// Pack the number of entries
	packer.PackUint64(uint64(len(p.Entries)))
	
	// Pack each entry
	for _, entry := range p.Entries {
		packer.PackBytes(entry.Key)
		packer.PackBytes(entry.Value)
	}
	
	return packer.Bytes(), packer.Err()
}

// Unmarshal unmarshals a polynomial commitment from bytes
func (p *PolynomialCommitment) Unmarshal(src []byte) error {
	unpacker := codec.NewReader(src, consts.MaxInt)
	
	// Unpack the ID
	var idBytes []byte
	unpacker.UnpackFixedBytes(ids.IDLen, &idBytes)
	copy(p.Root[:], idBytes)
	
	// Unpack the coefficients
	unpacker.UnpackBytes(consts.MaxInt, false, &p.Coefficients)
	
	// Unpack the timestamp
	p.Timestamp = unpacker.UnpackUint64(false)
	
	return unpacker.Err()
}

// Verify verifies that a proof is valid for this commitment
func (p *PolynomialCommitment) Verify(proof *PolynomialProof) error {
	// Verify that the root matches
	if !bytes.Equal(p.Root[:], proof.Root[:]) {
		return ErrMismatchedRoot
	}
	
	// Verify that the timestamp is consistent
	if p.Timestamp != proof.Timestamp {
		return fmt.Errorf("%w: timestamp mismatch", ErrInvalidProof)
	}
	
	// Additional verification logic would go here
	// This would involve the "Accidental Computer" polynomial commitment verification logic
	
	return nil
}

// PersistCommitment persists a polynomial commitment to state
func PersistCommitment(view state.View, p *PolynomialCommitment) error {
	ctx := context.Background()
	key := []byte(fmt.Sprintf("%s%s", CommitmentKeyPrefix, p.Root.String()))
	
	// Marshal the commitment
	buf := make([]byte, p.MarshalLen())
	buf, err := p.Marshal(buf[:0])
	if err != nil {
		return err
	}
	
	// Store the commitment in state using a new view
	changes := make(map[string]maybe.Maybe[[]byte])
	changes[string(key)] = maybe.Some(buf)
	
	// Create a new view with our changes
	_, err = view.NewView(ctx, merkledb.ViewChanges{
		MapOps: changes,
	})
	return err
}

// GetCommitment retrieves a commitment from state
func GetCommitment(view state.Immutable, root ids.ID) (*PolynomialCommitment, error) {
	ctx := context.Background()
	key := []byte(fmt.Sprintf("%s%s", CommitmentKeyPrefix, root.String()))
	
	// Get the commitment from state
	val, err := view.GetValue(ctx, key)
	if err != nil {
		return nil, err
	}
	
	// Unmarshal the commitment
	p := &PolynomialCommitment{}
	if err := p.Unmarshal(val); err != nil {
		return nil, err
	}
	
	return p, nil
}

// VerifyProof verifies a polynomial proof against a commitment
func VerifyProof(commitment *PolynomialCommitment, proof *PolynomialProof) error {
	return commitment.Verify(proof)
}

// InitializeCodec initializes the codec for polynomial commitments
func InitializeCodec() {
	// Any codec registration would go here if needed
}
