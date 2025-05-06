// Copyright (C) 2025. All rights reserved.
// See the file LICENSE for licensing terms.

package tee

import (
	"context"
	"errors"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/hypersdk/attestation"
)

var (
	ErrInvalidAttestation       = errors.New("invalid TEE attestation")
	ErrUnsupportedTEEType       = errors.New("unsupported TEE type")
	ErrInvalidTimestamp         = errors.New("invalid timestamp")
	ErrInvalidSignature         = errors.New("invalid signature")
	ErrMismatchedInputHash      = errors.New("mismatched input hash")
	ErrMismatchedOutputHash     = errors.New("mismatched output hash")
	ErrCrossAttestationRequired = errors.New("cross attestation required")
	ErrInvalidCommitment        = errors.New("invalid polynomial commitment")
)

// Verifier defines interface for verifying TEE attestations
type Verifier interface {
	// VerifyAttestation verifies a single TEE attestation
	VerifyAttestation(ctx context.Context, attestation *attestation.TEEAttestation) error

	// VerifyBatchAttestations verifies multiple attestations together (optimization)
	VerifyBatchAttestations(ctx context.Context, attestations []*attestation.TEEAttestation) error

	// VerifyTransaction verifies a transaction with TEE attestation
	VerifyTransaction(ctx context.Context, txID ids.ID, txBytes []byte, att *attestation.TEEAttestation) error

	// VerifyPolynomialCommitment verifies polynomial commitment proofs
	VerifyPolynomialCommitment(ctx context.Context, commitment []byte, proof []byte) error
}

// BlockVerifier interface for verifying block-level attestations
type BlockVerifier interface {
	// VerifyBlockAttestation verifies a block-level TEE attestation
	VerifyBlockAttestation(ctx context.Context, blockID ids.ID, attestation *attestation.TEEAttestation, commitment []byte) error
}

// Config configuration for the TEE verifier
type Config struct {
	// RequireCrossAttestation specifies whether cross-attestation is required
	RequireCrossAttestation bool `json:"requireCrossAttestation"`

	// SupportedTEETypes specifies which TEE types are supported
	SupportedTEETypes []attestation.TEEType `json:"supportedTEETypes"`

	// MaxTimestampDrift maximum acceptable timestamp drift in milliseconds
	MaxTimestampDrift int64 `json:"maxTimestampDrift"`

	// VerifierEndpoint endpoint for remote attestation verification
	VerifierEndpoint string `json:"verifierEndpoint"`
}

// TimestampVerifier interface for verifying timestamps
type TimestampVerifier interface {
	// VerifyTimestamp verifies the timestamp and its signature
	VerifyTimestamp(ctx context.Context, timestamp int64, signature []byte) error
}

// CoordinatorTimestampVerifier implements TimestampVerifier using coordinator
// timestamps (simpler implementation for initial phase)
type CoordinatorTimestampVerifier struct {
	// CoordinatorURL URL of the coordinator
	CoordinatorURL string

	// CoordinatorPublicKey public key of the coordinator (for verification)
	CoordinatorPublicKey []byte
}
