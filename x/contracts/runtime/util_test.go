// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"errors"
	"os"
	"path/filepath"
	"strings"

	"github.com/ava-labs/avalanchego/ids"
	"github.com/ava-labs/avalanchego/utils/logging"

	"github.com/ava-labs/hypersdk/codec"
	"github.com/ava-labs/hypersdk/state"
	"github.com/ava-labs/hypersdk/x/contracts/test"
)

type TestStateManager struct {
	ContractManager *ContractStateManager
	Balances        map[codec.Address]uint64
}

func (t TestStateManager) GetAccountContract(ctx context.Context, account codec.Address) (ContractID, error) {
	return t.ContractManager.GetAccountContract(ctx, account)
}

func (t TestStateManager) GetContractBytes(ctx context.Context, contractID ContractID) ([]byte, error) {
	return t.ContractManager.GetContractBytes(ctx, contractID)
}

func compileContract(contractName string) ([]byte, error) {
	if err := test.CompileTest(contractName); err != nil {
		return nil, err
	}
	dir, err := os.Getwd()
	if err != nil {
		return nil, err
	}

	contractName = strings.ReplaceAll(contractName, "-", "_")
	contractBytes, err := os.ReadFile(filepath.Join(dir, "/target/wasm32-unknown-unknown/release/"+contractName+".wasm"))
	if err != nil {
		return nil, err
	}

	return contractBytes, nil
}

func (t TestStateManager) SetContractBytes(ctx context.Context, contractID ContractID, contractBytes []byte) error {
	return t.ContractManager.SetContractBytes(ctx, contractID, contractBytes)
}

func (t TestStateManager) CompileAndSetContract(contractID ContractID, contractName string) error {
	contractBytes, err := compileContract(contractName)
	if err != nil {
		return err
	}
	return t.SetContractBytes(context.Background(), contractID, contractBytes)
}

func (t TestStateManager) NewAccountWithContract(_ context.Context, contractID ContractID, _ []byte) (codec.Address, error) {
	return t.ContractManager.NewAccountWithContract(context.Background(), contractID, []byte{})
}

func (t TestStateManager) SetAccountContract(_ context.Context, account codec.Address, contractID ContractID) error {
	return t.ContractManager.SetAccountContract(context.Background(), account, contractID)
}

func (t TestStateManager) GetBalance(_ context.Context, address codec.Address) (uint64, error) {
	if balance, ok := t.Balances[address]; ok {
		return balance, nil
	}
	return 0, nil
}

func (t TestStateManager) TransferBalance(ctx context.Context, from codec.Address, to codec.Address, amount uint64) error {
	balance, err := t.GetBalance(ctx, from)
	if err != nil {
		return err
	}
	if balance < amount {
		return errors.New("insufficient balance")
	}
	t.Balances[from] -= amount
	t.Balances[to] += amount
	return nil
}

func (t TestStateManager) GetContractState(address codec.Address) state.Mutable {
	return t.ContractManager.GetContractState(address)
}

var _ state.Mutable = (*prefixedState)(nil)

type prefixedState struct {
	address codec.Address
	inner   state.Mutable
}

func (p *prefixedState) GetValue(ctx context.Context, key []byte) (value []byte, err error) {
	return p.inner.GetValue(ctx, prependAccountToKey(p.address, key))
}

func (p *prefixedState) Insert(ctx context.Context, key []byte, value []byte) error {
	return p.inner.Insert(ctx, prependAccountToKey(p.address, key), value)
}

func (p *prefixedState) Remove(ctx context.Context, key []byte) error {
	return p.inner.Remove(ctx, prependAccountToKey(p.address, key))
}

// prependAccountToKey makes the key relative to the account
func prependAccountToKey(account codec.Address, key []byte) []byte {
	result := make([]byte, len(account)+len(key)+1)
	copy(result, account[:])
	copy(result[len(account):], "/")
	copy(result[len(account)+1:], key)
	return result
}

type testRuntime struct {
	Context      context.Context
	callContext  CallContext
	StateManager StateManager
}

func (t *testRuntime) WithStateManager(manager StateManager) *testRuntime {
	t.callContext = t.callContext.WithStateManager(manager)
	return t
}

func (t *testRuntime) WithActor(address codec.Address) *testRuntime {
	t.callContext = t.callContext.WithActor(address)
	return t
}

func (t *testRuntime) WithFunction(s string) *testRuntime {
	t.callContext = t.callContext.WithFunction(s)
	return t
}

func (t *testRuntime) WithContract(address codec.Address) *testRuntime {
	t.callContext = t.callContext.WithContract(address)
	return t
}

func (t *testRuntime) WithFuel(u uint64) *testRuntime {
	t.callContext = t.callContext.WithFuel(u)
	return t
}

func (t *testRuntime) WithParams(bytes []byte) *testRuntime {
	t.callContext = t.callContext.WithParams(bytes)
	return t
}

func (t *testRuntime) WithHeight(height uint64) *testRuntime {
	t.callContext = t.callContext.WithHeight(height)
	return t
}

func (t *testRuntime) WithActionID(actionID ids.ID) *testRuntime {
	t.callContext = t.callContext.WithActionID(actionID)
	return t
}

func (t *testRuntime) WithTimestamp(ts uint64) *testRuntime {
	t.callContext = t.callContext.WithTimestamp(ts)
	return t
}

func (t *testRuntime) WithValue(value uint64) *testRuntime {
	t.callContext = t.callContext.WithValue(value)
	return t
}

// AddContract compiles [contractName] and sets the bytes in the state manager
func (t *testRuntime) AddContract(contractID ContractID, account codec.Address, contractName string) error {
	err := t.StateManager.(TestStateManager).CompileAndSetContract(contractID, contractName)
	if err != nil {
		return err
	}
	return t.StateManager.(TestStateManager).SetAccountContract(t.Context, account, contractID)
}

func (t *testRuntime) CallContract(contract codec.Address, function string, params []byte) ([]byte, error) {
	return t.callContext.CallContract(
		t.Context,
		&CallInfo{
			Contract:     contract,
			State:        t.StateManager,
			FunctionName: function,
			Params:       params,
		})
}

func newTestRuntime(ctx context.Context) *testRuntime {
	cfg := NewConfig()
	log := logging.NoLog{}
	runtime := NewRuntime(cfg, log)

	stateManager := TestStateManager{
		ContractManager: NewContractStateManager(test.NewTestDB(), []byte{}),
		Balances:       map[codec.Address]uint64{},
	}

	return &testRuntime{
		Context:      ctx,
		callContext:  runtime.WithDefaults(CallInfo{Fuel: 1000000000}),
		StateManager: stateManager,
	}
}

func (t *testRuntime) newTestContract(contract string) (*testContract, error) {
	id := ids.GenerateTestID()
	account := codec.CreateAddress(0, id)
	stringedID := string(id[:])
	testContract := &testContract{
		ID:      ContractID(stringedID),
		Address: account,
		Runtime: t,
	}

	err := t.AddContract(ContractID(stringedID), account, contract)
	if err != nil {
		return nil, err
	}

	return testContract, nil
}

type testContract struct {
	ID      ContractID
	Address codec.Address
	Runtime *testRuntime
}

func (t *testContract) Call(function string, params ...interface{}) ([]byte, error) {
	args := test.SerializeParams(params...)
	return t.CallWithSerializedParams(function, args)
}

func (t *testContract) CallWithSerializedParams(function string, params []byte) ([]byte, error) {
	return t.Runtime.CallContract(
		t.Address,
		function,
		params)
}

func (t *testContract) WithStateManager(manager StateManager) *testContract {
	t.Runtime = t.Runtime.WithStateManager(manager)
	return t
}

func (t *testContract) WithActor(address codec.Address) *testContract {
	t.Runtime = t.Runtime.WithActor(address)
	return t
}

func (t *testContract) WithFunction(s string) *testContract {
	t.Runtime = t.Runtime.WithFunction(s)
	return t
}

func (t *testContract) WithContract(address codec.Address) *testContract {
	t.Runtime = t.Runtime.WithContract(address)
	return t
}

func (t *testContract) WithFuel(u uint64) *testContract {
	t.Runtime = t.Runtime.WithFuel(u)
	return t
}

func (t *testContract) WithParams(bytes []byte) *testContract {
	t.Runtime = t.Runtime.WithParams(bytes)
	return t
}

func (t *testContract) WithHeight(height uint64) *testContract {
	t.Runtime = t.Runtime.WithHeight(height)
	return t
}

func (t *testContract) WithActionID(actionID ids.ID) *testContract {
	t.Runtime = t.Runtime.WithActionID(actionID)
	return t
}

func (t *testContract) WithTimestamp(ts uint64) *testContract {
	t.Runtime = t.Runtime.WithTimestamp(ts)
	return t
}

func (t *testContract) WithValue(value uint64) *testContract {
	t.Runtime = t.Runtime.WithValue(value)
	return t
}

func into[T any](data []byte) T {
	result, err := Deserialize[T](data)
	if err != nil {
		panic(err.Error())
	}
	return *result
}
