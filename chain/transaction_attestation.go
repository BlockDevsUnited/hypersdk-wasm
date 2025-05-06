// Copyright (C) 2025. All rights reserved.
// See the file LICENSE for licensing terms.

package chain

import (
	att "github.com/ava-labs/hypersdk/attestation"
	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/consts"
)

// AttestedTransaction extends Transaction with TEE attestation support
type AttestedTransaction struct {
	*Transaction

	// TEE attestation proving correct execution
	Attestation *att.TEEAttestation `json:"attestation,omitempty"`
}

// NewAttestedTransaction creates a Transaction with TEE attestation
func NewAttestedTransaction(tx *Transaction, attestation *att.TEEAttestation) *AttestedTransaction {
	return &AttestedTransaction{
		Transaction: tx,
		Attestation: attestation,
	}
}

// Marshal serializes the AttestedTransaction to bytes
func (a *AttestedTransaction) Marshal(p *codec.Packer) error {
	// First marshal the base transaction
	if err := a.Transaction.Marshal(p); err != nil {
		return err
	}

	// Then marshal attestation (or nil marker if none)
	if a.Attestation == nil {
		p.PackBool(false)
	} else {
		p.PackBool(true)
		if err := a.Attestation.Marshal(p); err != nil {
			return err
		}
	}

	return p.Err()
}

// MarshalBytes returns the byte representation of the attested transaction
func (a *AttestedTransaction) MarshalBytes() ([]byte, error) {
	size := a.Transaction.Size()
	if a.Attestation != nil {
		size += 1 + a.Attestation.Size() // bool + attestation size
	} else {
		size += 1 // just bool
	}

	p := codec.NewWriter(size, consts.NetworkSizeLimit)
	if err := a.Marshal(p); err != nil {
		return nil, err
	}
	return p.Bytes(), p.Err()
}

// UnmarshalAttestedTransaction deserializes an AttestedTransaction from bytes
func UnmarshalAttestedTransaction(
	p *codec.Packer,
	actionRegistry *codec.TypeParser[Action],
	authRegistry *codec.TypeParser[Auth],
) (*AttestedTransaction, error) {
	// First unmarshal the base transaction
	tx, err := UnmarshalTx(p, actionRegistry, authRegistry)
	if err != nil {
		return nil, err
	}

	// Then unmarshal attestation if present
	hasAttestation := p.UnpackBool()
	var teeAttestation *att.TEEAttestation
	if hasAttestation {
		teeAttestation, err = att.UnmarshalTEEAttestation(p)
		if err != nil {
			return nil, err
		}
	}

	return &AttestedTransaction{
		Transaction: tx,
		Attestation: teeAttestation,
	}, nil
}

// IsAttested returns true if this transaction has a TEE attestation
func (a *AttestedTransaction) IsAttested() bool {
	return a.Attestation != nil
}

// GetAttestation returns the TEE attestation for this transaction
func (a *AttestedTransaction) GetAttestation() *att.TEEAttestation {
	return a.Attestation
}
