// Copyright (C) 2025. All rights reserved.
// See the file LICENSE for licensing terms.

package chain

import (
	"errors"
	"fmt"
	"time"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/hypersdk/attestation"
	"github.com/ava-labs/hypersdk/codec"
)

// Errors returned by attestation functions
var (
	ErrInvalidTimestamp = errors.New("invalid attestation timestamp")
)

// TEEType represents the type of Trusted Execution Environment
type TEEType uint8

const (
	TEETypeUnknown TEEType = 0
	TEETypeSGX     TEEType = 1
	TEETypeSEV     TEEType = 2
	TEETypeTDX     TEEType = 3
)

func (t TEEType) String() string {
	switch t {
	case TEETypeSGX:
		return "SGX"
	case TEETypeSEV:
		return "SEV"
	case TEETypeTDX:
		return "TDX"
	default:
		return "Unknown"
	}
}

// TEEAttestation represents hardware-based proof of correct execution from a TEE
type TEEAttestation struct {
	// Type of TEE that generated this attestation
	Type TEEType `json:"type"`

	// Hash of the input data that was executed in the TEE
	InputHash ids.ID `json:"inputHash"`

	// Hash of the output data produced by the TEE execution
	OutputHash ids.ID `json:"outputHash"`

	// Attestation report from the TEE (format depends on TEE type)
	Report []byte `json:"report"`

	// Signature over the attestation report by the TEE's attestation key
	Signature []byte `json:"signature"`

	// Timestamp of when this attestation was generated
	// (signed by timeserver in production, currently from coordinator)
	Timestamp int64 `json:"timestamp"`

	// Optional signature over the timestamp by a trusted timeserver
	TimestampSignature []byte `json:"timestampSignature,omitempty"`

	// For cross-attestation verification, this contains proof
	// that other TEE types verified this execution
	CrossAttestations []*CrossAttestation `json:"crossAttestations,omitempty"`
}

// CrossAttestation represents an attestation from a different TEE type
// verifying the same execution
type CrossAttestation struct {
	// Type of TEE that provided this cross-attestation
	Type TEEType `json:"type"`

	// Signature over the primary attestation by this TEE
	Signature []byte `json:"signature"`
}

// Size returns the size of this attestation when serialized
func (a *TEEAttestation) Size() int {
	size := 1 + // Type
		ids.IDLen + // InputHash
		ids.IDLen + // OutputHash
		codec.BytesLen(a.Report) +
		codec.BytesLen(a.Signature) +
		8 + // Timestamp
		codec.BytesLen(a.TimestampSignature)

	for _, ca := range a.CrossAttestations {
		size += 1 + codec.BytesLen(ca.Signature) // Type + Signature
	}
	return size
}

// Marshal serializes the TEEAttestation to bytes
func (a *TEEAttestation) Marshal(p *codec.Packer) error {
	p.PackByte(uint8(a.Type))
	p.PackID(a.InputHash)
	p.PackID(a.OutputHash)
	p.PackBytes(a.Report)
	p.PackBytes(a.Signature)
	p.PackInt64(a.Timestamp)
	p.PackBytes(a.TimestampSignature)

	p.PackInt(uint32(len(a.CrossAttestations)))
	for _, ca := range a.CrossAttestations {
		p.PackByte(uint8(ca.Type))
		p.PackBytes(ca.Signature)
	}
	return p.Err()
}

// UnmarshalTEEAttestation deserializes bytes into a TEEAttestation
func UnmarshalTEEAttestation(p *codec.Packer) (*TEEAttestation, error) {
	var a TEEAttestation
	a.Type = TEEType(p.UnpackByte())

	p.UnpackID(false, &a.InputHash)
	p.UnpackID(false, &a.OutputHash)

	p.UnpackBytes(-1, true, &a.Report)
	p.UnpackBytes(-1, true, &a.Signature)
	a.Timestamp = p.UnpackInt64(false)
	p.UnpackBytes(-1, true, &a.TimestampSignature)

	crossCount := p.UnpackInt(false)
	if crossCount > 0 {
		a.CrossAttestations = make([]*CrossAttestation, crossCount)
		for i := uint32(0); i < crossCount; i++ {
			ca := &CrossAttestation{
				Type: TEEType(p.UnpackByte()),
			}
			p.UnpackBytes(-1, true, &ca.Signature)
			a.CrossAttestations[i] = ca
		}
	}

	if err := p.Err(); err != nil {
		return nil, err
	}
	return &a, nil
}

// VerifyTimestamp checks if the attestation timestamp is within
// acceptable bounds compared to the current time
func (a *TEEAttestation) VerifyTimestamp() error {
	now := time.Now().UnixMilli()
	
	// Ensure timestamp is not too far in the future
	if a.Timestamp > now+attestation.MaxTimeWindow {
		return fmt.Errorf("%w: timestamp too far in future", ErrInvalidTimestamp)
	}
	
	// Ensure timestamp is not too far in the past
	if a.Timestamp < now-attestation.MaxTimeWindow {
		return fmt.Errorf("%w: timestamp too far in past", ErrInvalidTimestamp)
	}
	
	return nil
}

// String returns a string representation of the attestation
func (a *TEEAttestation) String() string {
	return fmt.Sprintf("TEEAttestation(Type=%s, InputHash=%s, OutputHash=%s, Timestamp=%d)",
		a.Type, a.InputHash, a.OutputHash, a.Timestamp)
}
