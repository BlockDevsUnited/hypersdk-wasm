// Copyright (C) 2024, Aristo Technologies. All rights reserved.
// See the file LICENSE for licensing terms.

package regional

import (
	"context"
	"errors"
	"fmt"
	"sync"
	"time"

	"go.uber.org/zap"
)

// ErrorCategory represents the category of an error
type ErrorCategory int

const (
	// NetworkError represents a network communication error
	NetworkError ErrorCategory = iota
	
	// TimeoutError represents a timeout error
	TimeoutError
	
	// ValidationError represents a validation error
	ValidationError
	
	// StateError represents a state-related error
	StateError
	
	// RegulationError represents a regulatory compliance error
	RegulationError
	
	// SecurityError represents a security-related error
	SecurityError
	
	// InternalError represents an internal system error
	InternalError
)

// CrossRegionError wraps an error with additional context
type CrossRegionError struct {
	Category ErrorCategory
	Region   string
	Message  string
	Err      error
	TxID     string
	// Critical errors require immediate attention and may have regulatory implications
	Critical bool
	// RetryAfter suggests when to retry the operation
	RetryAfter time.Duration
}

// Error implements the error interface
func (e CrossRegionError) Error() string {
	if e.Err != nil {
		return fmt.Sprintf("%s: %v (region: %s, txID: %s)", e.Message, e.Err, e.Region, e.TxID)
	}
	return fmt.Sprintf("%s (region: %s, txID: %s)", e.Message, e.Region, e.TxID)
}

// Unwrap returns the wrapped error
func (e CrossRegionError) Unwrap() error {
	return e.Err
}

// NewNetworkError creates a new network error
func NewNetworkError(region string, txID string, message string, err error) error {
	return CrossRegionError{
		Category:   NetworkError,
		Region:     region,
		Message:    message,
		Err:        err,
		TxID:       txID,
		Critical:   false,
		RetryAfter: 100 * time.Millisecond, // Align with "100ms and regulated" value proposition
	}
}

// NewTimeoutError creates a new timeout error
func NewTimeoutError(region string, txID string, message string) error {
	return CrossRegionError{
		Category:   TimeoutError,
		Region:     region,
		Message:    message,
		Err:        context.DeadlineExceeded,
		TxID:       txID,
		Critical:   false,
		RetryAfter: 250 * time.Millisecond,
	}
}

// NewValidationError creates a new validation error
func NewValidationError(region string, txID string, message string, err error) error {
	return CrossRegionError{
		Category:   ValidationError,
		Region:     region,
		Message:    message,
		Err:        err,
		TxID:       txID,
		Critical:   false,
		RetryAfter: 500 * time.Millisecond,
	}
}

// NewRegulationError creates a new regulatory error
func NewRegulationError(region string, txID string, message string) error {
	return CrossRegionError{
		Category:   RegulationError,
		Region:     region,
		Message:    message,
		TxID:       txID,
		Critical:   true, // Regulatory errors are always critical
		RetryAfter: 0,    // Don't retry regulatory errors
	}
}

// RetryPolicy defines how operations are retried
type RetryPolicy struct {
	// MaxAttempts is the maximum number of retry attempts
	MaxAttempts int
	
	// InitialBackoff is the initial backoff duration
	InitialBackoff time.Duration
	
	// MaxBackoff is the maximum backoff duration
	MaxBackoff time.Duration
	
	// BackoffFactor is the factor by which backoff increases
	BackoffFactor float64
	
	// NonRetryableCategories are error categories that shouldn't be retried
	NonRetryableCategories []ErrorCategory
}

// DefaultRetryPolicy returns a default retry policy
func DefaultRetryPolicy() RetryPolicy {
	return RetryPolicy{
		MaxAttempts:           3,
		InitialBackoff:        100 * time.Millisecond,
		MaxBackoff:            1 * time.Second,
		BackoffFactor:         2.0,
		NonRetryableCategories: []ErrorCategory{RegulationError, SecurityError},
	}
}

// CircuitBreaker implements the circuit breaker pattern
type CircuitBreaker struct {
	failureThreshold    int
	resetTimeout        time.Duration
	failures            int
	lastFailure         time.Time
	state               string
	mutex               sync.RWMutex
	logger              *zap.Logger
	successThreshold    int
	successes           int
	shouldTripPredicate func(err error) bool
}

const (
	CircuitClosed   = "CLOSED"    // Circuit is closed and operating normally
	CircuitOpen     = "OPEN"      // Circuit is open and failing fast
	CircuitHalfOpen = "HALF_OPEN" // Circuit is half-open and testing if it can be closed
)

// NewCircuitBreaker creates a new circuit breaker
func NewCircuitBreaker(failureThreshold int, resetTimeout time.Duration, 
	shouldTripPredicate func(err error) bool, logger *zap.Logger) *CircuitBreaker {
	
	return &CircuitBreaker{
		failureThreshold:    failureThreshold,
		resetTimeout:        resetTimeout,
		failures:            0,
		lastFailure:         time.Time{},
		state:               CircuitClosed,
		logger:              logger,
		successThreshold:    1,
		successes:           0,
		shouldTripPredicate: shouldTripPredicate,
	}
}

// WithSuccessThreshold sets the success threshold needed to close the circuit
func (cb *CircuitBreaker) WithSuccessThreshold(threshold int) *CircuitBreaker {
	cb.successThreshold = threshold
	return cb
}

// Execute executes a function with circuit breaker protection
func (cb *CircuitBreaker) Execute(operation func() error) error {
	cb.mutex.RLock()
	state := cb.state
	cb.mutex.RUnlock()
	
	// If circuit is open, check if we can try again
	if state == CircuitOpen {
		cb.mutex.RLock()
		lastFailure := cb.lastFailure
		cb.mutex.RUnlock()
		
		if time.Since(lastFailure) > cb.resetTimeout {
			cb.mutex.Lock()
			cb.state = CircuitHalfOpen
			cb.mutex.Unlock()
			
			cb.logger.Info("Circuit changed to half-open state")
		} else {
			return errors.New("circuit breaker is open")
		}
	}
	
	// Execute the operation
	err := operation()
	
	// Handle the result
	if err != nil {
		// Should this error trip the circuit breaker?
		if cb.shouldTripPredicate != nil && !cb.shouldTripPredicate(err) {
			return err // Error doesn't trip the circuit breaker
		}
		
		cb.mutex.Lock()
		defer cb.mutex.Unlock()
		
		cb.failures++
		cb.successes = 0
		cb.lastFailure = time.Now()
		
		if cb.state == CircuitHalfOpen || cb.failures >= cb.failureThreshold {
			cb.state = CircuitOpen
			cb.logger.Warn("Circuit changed to open state", 
				zap.Error(err),
				zap.Int("failures", cb.failures),
			)
		}
		
		return err
	}
	
	// Operation succeeded
	cb.mutex.Lock()
	defer cb.mutex.Unlock()
	
	// Reset failures only if we're in closed state
	if cb.state == CircuitClosed {
		cb.failures = 0
		return nil
	}
	
	// If we're in half-open state, count successes
	if cb.state == CircuitHalfOpen {
		cb.successes++
		
		// If we've reached the success threshold, close the circuit
		if cb.successes >= cb.successThreshold {
			cb.state = CircuitClosed
			cb.failures = 0
			cb.successes = 0
			cb.logger.Info("Circuit changed to closed state")
		}
	}
	
	return nil
}

// GetState returns the current state of the circuit breaker
func (cb *CircuitBreaker) GetState() string {
	cb.mutex.RLock()
	defer cb.mutex.RUnlock()
	return cb.state
}

// RetryWithBackoff retries an operation with exponential backoff
func RetryWithBackoff(ctx context.Context, operation func() error, 
	policy RetryPolicy, logger *zap.Logger) error {
	
	var lastErr error
	backoff := policy.InitialBackoff
	
	// Try the initial attempt plus retries
	for attempt := 0; attempt <= policy.MaxAttempts; attempt++ {
		// Check for context cancellation
		select {
		case <-ctx.Done():
			return NewTimeoutError("", "", "Operation timed out due to context cancellation")
		default:
		}
		
		// Skip backoff on first attempt
		if attempt > 0 {
			logger.Debug("Retrying operation",
				zap.Int("attempt", attempt),
				zap.Duration("backoff", backoff),
				zap.Error(lastErr),
			)
			
			// Wait according to backoff policy
			select {
			case <-ctx.Done():
				return NewTimeoutError("", "", "Operation timed out during retry backoff")
			case <-time.After(backoff):
			}
			
			// Increase backoff for next attempt
			backoff = time.Duration(float64(backoff) * policy.BackoffFactor)
			if backoff > policy.MaxBackoff {
				backoff = policy.MaxBackoff
			}
		}
		
		// Execute operation
		err := operation()
		if err == nil {
			return nil // Success
		}
		
		lastErr = err
		
		// Check if this error should not be retried
		var crErr CrossRegionError
		if errors.As(err, &crErr) {
			for _, cat := range policy.NonRetryableCategories {
				if crErr.Category == cat {
					logger.Warn("Non-retryable error encountered",
						zap.Error(err),
						zap.Int("category", int(crErr.Category)),
					)
					return err
				}
			}
			
			// Use error-specific retry after time if available
			if crErr.RetryAfter > 0 {
				backoff = crErr.RetryAfter
			}
		}
		
		// Last attempt - return the error
		if attempt == policy.MaxAttempts {
			logger.Warn("Operation failed after max retry attempts",
				zap.Int("maxAttempts", policy.MaxAttempts),
				zap.Error(err),
			)
			return lastErr
		}
	}
	
	return lastErr
}
