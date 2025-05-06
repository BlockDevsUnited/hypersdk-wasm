// Copyright (C) 2025. All rights reserved.
// See the file LICENSE for licensing terms.

package tee

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strings"
	"time"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/hypersdk/attestation"
	"github.com/ava-labs/hypersdk/utils"
)

// MeshVerifier implements the Verifier interface using the TEE mesh network
type MeshVerifier struct {
	// Configuration for the verifier
	config Config

	// Timestamp verifier for validating timestamps
	timestampVerifier TimestampVerifier

	// HTTP client for making requests to the TEE mesh
	client *http.Client
}

// VerifierResponse represents the response from the TEE mesh verification endpoint
type VerifierResponse struct {
	Success bool   `json:"success"`
	Error   string `json:"error,omitempty"`

	// For polynomial commitment verification
	IsValid bool `json:"isValid"`
}

// NewMeshVerifier creates a new TEE verifier that connects to the mesh network
func NewMeshVerifier(config Config, timestampVerifier TimestampVerifier) *MeshVerifier {
	return &MeshVerifier{
		config:            config,
		timestampVerifier: timestampVerifier,
		client: &http.Client{
			Timeout: 5 * time.Second,
		},
	}
}

// VerifyAttestation verifies a single TEE attestation using the mesh network
func (m *MeshVerifier) VerifyAttestation(ctx context.Context, attestation *attestation.TEEAttestation) error {
	// Check TEE type is supported
	if !m.isSupportedTEEType(attestation.Type) {
		return fmt.Errorf("%w: %s", ErrUnsupportedTEEType, attestation.Type)
	}

	// Verify timestamp (either locally or using the timestamp verifier)
	if err := attestation.VerifyTimestamp(); err != nil {
		return err
	}

	// If a timestamp signature is present, verify it
	if len(attestation.TimestampSignature) > 0 {
		if err := m.timestampVerifier.VerifyTimestamp(ctx, attestation.Timestamp, attestation.TimestampSignature); err != nil {
			return fmt.Errorf("%w: %w", ErrInvalidTimestamp, err)
		}
	}

	// Check if cross-attestation is required but missing
	if m.config.RequireCrossAttestation && len(attestation.CrossAttestations) == 0 {
		return ErrCrossAttestationRequired
	}

	// Make the verification request to the TEE mesh network
	attestationBytes, err := json.Marshal(attestation)
	if err != nil {
		return fmt.Errorf("failed to marshal attestation: %w", err)
	}

	resp, err := m.client.Post(
		strings.TrimSuffix(m.config.VerifierEndpoint, "/")+"/verify_attestation",
		"application/json",
		strings.NewReader(string(attestationBytes)),
	)
	if err != nil {
		return fmt.Errorf("failed to connect to verifier: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("verifier returned non-OK status: %d", resp.StatusCode)
	}

	var verifierResp VerifierResponse
	if err := json.NewDecoder(resp.Body).Decode(&verifierResp); err != nil {
		return fmt.Errorf("failed to decode verifier response: %w", err)
	}

	if !verifierResp.Success {
		return fmt.Errorf("%w: %s", ErrInvalidAttestation, verifierResp.Error)
	}

	return nil
}

// VerifyBatchAttestations verifies multiple attestations together (optimization)
func (m *MeshVerifier) VerifyBatchAttestations(ctx context.Context, attestations []*attestation.TEEAttestation) error {
	// For simplicity in the initial implementation, just verify each attestation
	// In a production implementation, this would batch the verification
	for _, attestation := range attestations {
		if err := m.VerifyAttestation(ctx, attestation); err != nil {
			return err
		}
	}
	return nil
}

// VerifyTransaction verifies a transaction with TEE attestation
func (m *MeshVerifier) VerifyTransaction(ctx context.Context, txID ids.ID, txBytes []byte, att *attestation.TEEAttestation) error {
	if att == nil {
		return fmt.Errorf("%w: transaction has no attestation", ErrInvalidAttestation)
	}

	// Verify the attestation
	if err := m.VerifyAttestation(ctx, att); err != nil {
		return err
	}

	// Verify that the transaction hash matches the attestation input hash
	txHash := utils.ToID(txBytes)
	if txHash != att.InputHash {
		return fmt.Errorf("%w: transaction hash %s doesn't match attestation input hash %s", 
			ErrMismatchedInputHash, txHash, att.InputHash)
	}

	return nil
}

// VerifyPolynomialCommitment verifies polynomial commitment proofs
func (m *MeshVerifier) VerifyPolynomialCommitment(ctx context.Context, commitment []byte, proof []byte) error {
	// Make the verification request to the TEE mesh network
	requestData := map[string]interface{}{
		"commitment": fmt.Sprintf("%x", commitment),
		"proof":      fmt.Sprintf("%x", proof),
	}

	requestBytes, err := json.Marshal(requestData)
	if err != nil {
		return fmt.Errorf("failed to marshal commitment data: %w", err)
	}

	resp, err := m.client.Post(
		strings.TrimSuffix(m.config.VerifierEndpoint, "/")+"/verify_polynomial_commitment",
		"application/json",
		strings.NewReader(string(requestBytes)),
	)
	if err != nil {
		return fmt.Errorf("failed to connect to verifier: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK {
		return fmt.Errorf("verifier returned non-OK status: %d", resp.StatusCode)
	}

	var verifierResp VerifierResponse
	if err := json.NewDecoder(resp.Body).Decode(&verifierResp); err != nil {
		return fmt.Errorf("failed to decode verifier response: %w", err)
	}

	if !verifierResp.Success || !verifierResp.IsValid {
		return fmt.Errorf("%w: %s", ErrInvalidCommitment, verifierResp.Error)
	}

	return nil
}

// isSupportedTEEType checks if the TEE type is supported by this verifier
func (m *MeshVerifier) isSupportedTEEType(teeType attestation.TEEType) bool {
	for _, supportedType := range m.config.SupportedTEETypes {
		if teeType == supportedType {
			return true
		}
	}
	return false
}
