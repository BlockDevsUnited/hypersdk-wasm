// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package testutils

import (
	"context"
	"errors"
	"sync"
)

// MockDB implements state.Mutable for testing
type MockDB struct {
	mu    sync.RWMutex
	store map[string][]byte
}

// NewMockDB creates a new mock database
func NewMockDB() *MockDB {
	return &MockDB{
		store: make(map[string][]byte),
	}
}

// Get returns a value from the mock database
func (m *MockDB) Get(ctx context.Context, key []byte) ([]byte, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()

	if value, ok := m.store[string(key)]; ok {
		return value, nil
	}
	return nil, nil
}

// Insert adds a value to the mock database
func (m *MockDB) Insert(ctx context.Context, key []byte, value []byte) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	m.store[string(key)] = value
	return nil
}

// Remove removes a value from the mock database
func (m *MockDB) Remove(ctx context.Context, key []byte) error {
	m.mu.Lock()
	defer m.mu.Unlock()

	delete(m.store, string(key))
	return nil
}

// GetValue returns the value for a key
func (m *MockDB) GetValue(ctx context.Context, key []byte) ([]byte, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	
	if val, ok := m.store[string(key)]; ok {
		return val, nil
	}
	return nil, errors.New("key not found")
}
