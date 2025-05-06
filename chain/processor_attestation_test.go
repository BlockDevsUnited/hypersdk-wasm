// Copyright (C) 2025. All rights reserved.
// See the file LICENSE for licensing terms.

package chain

import (
	"encoding/binary"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
)

// TestDualTEEAttestationFormats tests the core functionality of our dual TEE architecture:
// validating both SGX and SEV attestation formats in both length-prefixed and direct formats
func TestDualTEEAttestationFormats(t *testing.T) {
	tests := []struct {
		name        string
		report      []byte
		expectSGX   bool
		expectSEV   bool
		description string
	}{
		{
			name:        "SGX attestation - length prefixed",
			report:      createSGXReport(true),
			expectSGX:   true,
			expectSEV:   false,
			description: "SGX attestation with length prefix should be recognized",
		},
		{
			name:        "SEV attestation - length prefixed",
			report:      createSEVReport(true),
			expectSGX:   false,
			expectSEV:   true,
			description: "SEV attestation with length prefix should be recognized",
		},
		{
			name:        "SGX attestation - direct format",
			report:      createSGXReport(false),
			expectSGX:   true,
			expectSEV:   false,
			description: "SGX attestation with direct format should be recognized",
		},
		{
			name:        "SEV attestation - direct format",
			report:      createSEVReport(false),
			expectSGX:   false,
			expectSEV:   true, 
			description: "SEV attestation with direct format should be recognized",
		},
		{
			name:        "Invalid attestation format",
			report:      []byte("invalid attestation data"),
			expectSGX:   false,
			expectSEV:   false,
			description: "Invalid attestation format should be rejected",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Test both SGX and SEV format detection directly
			isSGX := isSGXReport(tt.report)
			isSEV := isSEVReport(tt.report)
			
			assert.Equal(t, tt.expectSGX, isSGX, "SGX detection should match expectation")
			assert.Equal(t, tt.expectSEV, isSEV, "SEV detection should match expectation")
		})
	}
}

// TestPolynomialCommitmentPattern tests the pattern matching for our polynomial commitment 
// system using a simplified approach
func TestPolynomialCommitmentPattern(t *testing.T) {
	tests := []struct {
		name        string
		report      []byte
		expectFound bool
		description string
	}{
		{
			name:        "SGX with commitment pattern",
			report:      createReportWithCommitment("SGX"),
			expectFound: true,
			description: "SGX attestation with commitment pattern should be recognized",
		},
		{
			name:        "SEV with commitment pattern",
			report:      createReportWithCommitment("SEV"),
			expectFound: true,
			description: "SEV attestation with commitment pattern should be recognized",
		},
		{
			name:        "Report without commitment pattern",
			report:      []byte("just some random data without commitment pattern"),
			expectFound: false,
			description: "Reports without commitment pattern should be rejected",
		},
	}
	
	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Test if we can detect the polynomial commitment pattern
			foundCommitment, foundProof := extractCommitmentFromReport(tt.report)
			
			if tt.expectFound {
				assert.NotEmpty(t, foundCommitment, "Commitment data should not be empty")
				assert.NotEmpty(t, foundProof, "Proof data should not be empty")
			} else {
				assert.Empty(t, foundCommitment, "Commitment data should be empty")
				assert.Empty(t, foundProof, "Proof data should be empty")
			}
		})
	}
}

// Helper functions for testing our dual TEE attestation system

// isSGXReport detects SGX attestation format
func isSGXReport(report []byte) bool {
	// Check for length-prefixed format first
	if len(report) > 4 {
		// Check if first 4 bytes might be a length prefix
		declaredLength := binary.LittleEndian.Uint32(report[0:4])
		
		// If the declared length matches remaining data, it's likely length-prefixed
		if declaredLength == uint32(len(report)-4) {
			// Check the magic numbers in the data after length prefix
			if len(report) >= 9 && string(report[4:8]) == "SGX_" && report[8] == byte(1) {
				return true
			}
		}
	}
	
	// Check for direct format (no length prefix)
	if len(report) >= 5 && string(report[0:4]) == "SGX_" && report[4] == byte(1) {
		return true
	}
	
	return false
}

// isSEVReport detects SEV attestation format
func isSEVReport(report []byte) bool {
	// Check for length-prefixed format first
	if len(report) > 4 {
		// Check if first 4 bytes might be a length prefix
		declaredLength := binary.LittleEndian.Uint32(report[0:4])
		
		// If the declared length matches remaining data, it's likely length-prefixed
		if declaredLength == uint32(len(report)-4) {
			// Check the magic numbers in the data after length prefix
			if len(report) >= 9 && string(report[4:8]) == "SEV_" && report[8] == byte(2) {
				return true
			}
		}
	}
	
	// Check for direct format (no length prefix)
	if len(report) >= 5 && string(report[0:4]) == "SEV_" && report[4] == byte(2) {
		return true
	}
	
	return false
}

// extractCommitmentFromReport looks for polynomial commitment pattern in attestation data
func extractCommitmentFromReport(report []byte) ([]byte, []byte) {
	// Look for commitment header
	commitmentHeader := []byte{0xAC, 0xC0, 0xAC, 0xC0} // Commitment header
	
	// Search for header in report
	headerIndex := -1
	for i := 0; i <= len(report)-len(commitmentHeader); i++ {
		if len(report) >= i+4 {
			if binary.LittleEndian.Uint32(report[i:i+4]) == binary.LittleEndian.Uint32(commitmentHeader) {
				headerIndex = i
				break
			}
		}
	}
	
	if headerIndex == -1 {
		return nil, nil
	}
	
	// Extract timestamp (8 bytes)
	if headerIndex+12 > len(report) {
		return nil, nil
	}
	
	// Extract commitment (16 bytes)
	if headerIndex+28 > len(report) {
		return nil, nil
	}
	commitment := report[headerIndex+12:headerIndex+28]
	
	// Extract proof (16 bytes)
	if headerIndex+44 > len(report) {
		return nil, nil
	}
	proof := report[headerIndex+28:headerIndex+44]
	
	return commitment, proof
}

// Create SGX report data for testing
func createSGXReport(lengthPrefixed bool) []byte {
	// Create basic SGX attestation data
	magic := []byte("SGX_")
	teeType := byte(1) // SGX type
	attestationData := []byte("sgx attestation data for testing")
	
	data := append(magic, teeType)
	data = append(data, attestationData...)
	
	if lengthPrefixed {
		// Add length prefix
		length := uint32(len(data))
		lengthBytes := make([]byte, 4)
		binary.LittleEndian.PutUint32(lengthBytes, length)
		
		prefixedData := append(lengthBytes, data...)
		return prefixedData
	}
	
	return data
}

// Create SEV report data for testing
func createSEVReport(lengthPrefixed bool) []byte {
	// Create basic SEV attestation data
	magic := []byte("SEV_")
	teeType := byte(2) // SEV type
	attestationData := []byte("sev attestation data for testing")
	
	data := append(magic, teeType)
	data = append(data, attestationData...)
	
	if lengthPrefixed {
		// Add length prefix
		length := uint32(len(data))
		lengthBytes := make([]byte, 4)
		binary.LittleEndian.PutUint32(lengthBytes, length)
		
		prefixedData := append(lengthBytes, data...)
		return prefixedData
	}
	
	return data
}

// Create test report with polynomial commitment pattern
func createReportWithCommitment(teeType string) []byte {
	// Convert tee type to magic prefix
	var magic []byte
	var typeCode byte
	
	if teeType == "SGX" {
		magic = []byte("SGX_")
		typeCode = 1
	} else if teeType == "SEV" {
		magic = []byte("SEV_")
		typeCode = 2
	}
	
	// Create basic report with magic + type
	data := append(magic, typeCode)
	
	// Add commitment header
	commitmentHeader := []byte{0xAC, 0xC0, 0xAC, 0xC0}
	data = append(data, commitmentHeader...)
	
	// Add timestamp
	timestamp := time.Now().Unix()
	timestampBytes := make([]byte, 8)
	binary.LittleEndian.PutUint64(timestampBytes, uint64(timestamp))
	data = append(data, timestampBytes...)
	
	// Add commitment (16 bytes)
	commitment := make([]byte, 16)
	for i := range commitment {
		commitment[i] = byte(i)
	}
	data = append(data, commitment...)
	
	// Add proof (16 bytes)
	proof := make([]byte, 16)
	for i := range proof {
		proof[i] = byte(i + 16)
	}
	data = append(data, proof...)
	
	return data
}
