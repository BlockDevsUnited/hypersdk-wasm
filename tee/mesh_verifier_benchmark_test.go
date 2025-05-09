package tee

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/hypersdk/attestation"
	"github.com/stretchr/testify/require"
)

// Benchmark test types (matches the test file)
type benchBatchVerificationRequest struct {
	Attestations     []*attestation.TEEAttestation `json:"attestations"`
	BatchID          string                        `json:"batchId,omitempty"`
	VerificationType string                        `json:"verificationType,omitempty"`
}

type benchVerificationResponse struct {
	Success bool                        `json:"success"`
	Results []benchVerificationResult   `json:"results"`
	Message string                      `json:"message,omitempty"`
}

type benchVerificationResult struct {
	Success bool   `json:"success"`
	ID      string `json:"id"`
	Error   string `json:"error,omitempty"`
}

// Mock verification server for benchmarks
func createBenchmarkServer(b *testing.B, latency time.Duration, failureRate float64) *httptest.Server {
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Simulate server processing time
		if latency > 0 {
			time.Sleep(latency)
		}

		// Parse request body
		var req benchBatchVerificationRequest
		err := json.NewDecoder(r.Body).Decode(&req)
		if err != nil {
			b.Fatalf("Failed to decode request: %v", err)
		}

		// Prepare response
		resp := benchVerificationResponse{
			Success: true,
			Results: make([]benchVerificationResult, len(req.Attestations)),
		}

		// Set results based on failure rate
		for i := range resp.Results {
			// Simple deterministic "random" check based on attestation index
			success := true
			if failureRate > 0 {
				// This is not truly random but sufficient for benchmarking
				if float64(i%100)/100 < failureRate {
					success = false
				}
			}

			resp.Results[i] = benchVerificationResult{
				Success: success,
				ID:      req.Attestations[i].InputHash.String(),
				Error:   "",
			}

			if !success {
				resp.Success = false
				resp.Results[i].Error = "Simulated verification failure"
			}
		}

		// Send response
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(resp)
	}))
}

// createTestAttestations generates a slice of test attestations of specified size
func createTestAttestations(size int) []*attestation.TEEAttestation {
	attestations := make([]*attestation.TEEAttestation, size)
	for i := 0; i < size; i++ {
		id1 := ids.GenerateTestID()
		id2 := ids.GenerateTestID()
		attestations[i] = &attestation.TEEAttestation{
			Type:        attestation.TEETypeSGX,
			InputHash:   id1,
			OutputHash:  id2,
			Timestamp:   time.Now().UnixNano(),
			Report:      []byte("test-report"),
			Signature:   []byte("test-signature"),
		}
	}
	return attestations
}

// createBenchmarkCircuitBreaker creates a circuit breaker for benchmarks
func createBenchmarkCircuitBreaker() *CircuitBreaker {
	return NewCircuitBreaker(100, 1, 1*time.Second)
}

// BenchmarkBatchVerification benchmarks batch verification with different batch sizes
func BenchmarkBatchVerification(b *testing.B) {
	// Create a test server that always returns success
	server := createBenchmarkServer(b, 0, 0)
	defer server.Close()

	// Batch sizes to benchmark
	batchSizes := []int{1, 10, 50, 100}

	for _, batchSize := range batchSizes {
		b.Run(fmt.Sprintf("BatchSize_%d", batchSize), func(b *testing.B) {
			// Create test data once
			attestations := createTestAttestations(batchSize)

			// Create config
			config := Config{
				VerifierEndpoint: server.URL,
				SupportedTEETypes: []attestation.TEEType{attestation.TEETypeSGX},
			}

			// Create verifier
			verifier := &MeshVerifier{
				config:         config,
				circuitBreaker: createBenchmarkCircuitBreaker(),
				client:         server.Client(),
				metrics:        &VerifierMetrics{},
			}

			// Reset timer before the benchmark loop
			b.ResetTimer()

			// Run benchmark
			for i := 0; i < b.N; i++ {
				err := verifier.batchVerifyAttestationGroup(context.Background(), attestations)
				require.NoError(b, err)
			}
		})
	}
}

// BenchmarkLatencyImpact benchmarks the impact of network latency
func BenchmarkLatencyImpact(b *testing.B) {
	// Test with different latencies
	latencies := []time.Duration{0, 10 * time.Millisecond, 50 * time.Millisecond, 100 * time.Millisecond}

	for _, latency := range latencies {
		b.Run("Latency_"+latency.String(), func(b *testing.B) {
			// Create a test server with the specified latency
			server := createBenchmarkServer(b, latency, 0)
			defer server.Close()

			// Create config
			config := Config{
				VerifierEndpoint: server.URL,
				SupportedTEETypes: []attestation.TEEType{attestation.TEETypeSGX},
			}

			// Create a mesh verifier for benchmarking
			verifier := &MeshVerifier{
				config:         config,
				circuitBreaker: createBenchmarkCircuitBreaker(),
				client:         server.Client(),
				metrics:        &VerifierMetrics{},
			}

			// Fixed batch size of 50 for latency tests
			attestations := createTestAttestations(50)

			b.ResetTimer()

			for i := 0; i < b.N; i++ {
				err := verifier.batchVerifyAttestationGroup(context.Background(), attestations)
				require.NoError(b, err)
			}
		})
	}
}

// BenchmarkFailureRateImpact benchmarks the impact of verification failures
func BenchmarkFailureRateImpact(b *testing.B) {
	// Test with different failure rates
	failureRates := []float64{0, 0.1, 0.25, 0.5, 0.75}

	for _, failureRate := range failureRates {
		b.Run(fmt.Sprintf("FailureRate_%.2f", failureRate), func(b *testing.B) {
			// Create a test server with the specified failure rate
			server := createBenchmarkServer(b, 1*time.Millisecond, failureRate)
			defer server.Close()

			// Create config
			config := Config{
				VerifierEndpoint: server.URL,
				SupportedTEETypes: []attestation.TEEType{attestation.TEETypeSGX},
			}

			// Create a mesh verifier for benchmarking
			verifier := &MeshVerifier{
				config:         config,
				circuitBreaker: createBenchmarkCircuitBreaker(),
				client:         server.Client(),
				metrics:        &VerifierMetrics{},
			}

			// Fixed batch size of 50 for failure rate tests
			attestations := createTestAttestations(50)

			b.ResetTimer()

			for i := 0; i < b.N; i++ {
				_ = verifier.batchVerifyAttestationGroup(context.Background(), attestations)
				// Ignore errors as we expect some failures based on the rate
			}
		})
	}
}

// BenchmarkConcurrentVerification benchmarks concurrent verification requests
func BenchmarkConcurrentVerification(b *testing.B) {
	// Create a test server
	server := createBenchmarkServer(b, 5*time.Millisecond, 0)
	defer server.Close()
	
	// Create config
	config := Config{
		VerifierEndpoint: server.URL,
		SupportedTEETypes: []attestation.TEEType{attestation.TEETypeSGX},
	}
	
	// Create a mesh verifier for benchmarking
	verifier := &MeshVerifier{
		config:         config,
		circuitBreaker: createBenchmarkCircuitBreaker(),
		client:         server.Client(),
		metrics:        &VerifierMetrics{},
	}
	
	// Fixed batch size
	attestations := createTestAttestations(20)
	
	b.ResetTimer()
	
	// Run with multiple goroutines
	b.RunParallel(func(pb *testing.PB) {
		for pb.Next() {
			err := verifier.batchVerifyAttestationGroup(context.Background(), attestations)
			require.NoError(b, err)
		}
	})
}

// BenchmarkThroughput measures attestation throughput
func BenchmarkThroughput(b *testing.B) {
	// Create a test server with minimal latency
	server := createBenchmarkServer(b, 1*time.Millisecond, 0)
	defer server.Close()
	
	// Create config
	config := Config{
		VerifierEndpoint: server.URL,
		SupportedTEETypes: []attestation.TEEType{attestation.TEETypeSGX},
	}
	
	// Create a mesh verifier for benchmarking
	verifier := &MeshVerifier{
		config:         config,
		circuitBreaker: createBenchmarkCircuitBreaker(),
		client:         server.Client(),
		metrics:        &VerifierMetrics{},
	}
	
	// Prepare a large batch of attestations
	batchSize := 100 // Reduced from 1000 for faster tests
	attestations := createTestAttestations(batchSize)
	
	// Run benchmark with a fixed number of iterations
	b.ResetTimer()
	b.SetBytes(int64(batchSize)) // Each iteration processes 'batchSize' attestations
	
	for i := 0; i < b.N; i++ {
		err := verifier.batchVerifyAttestationGroup(context.Background(), attestations)
		require.NoError(b, err)
	}
	
	// Report attestations per second in the output
	b.ReportMetric(float64(batchSize), "attestations/op")
}
