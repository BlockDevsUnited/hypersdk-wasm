// Copyright (C) 2024, Ava Labs, Inc. All rights reserved.
// See the file LICENSE for licensing terms.

package runtime

import (
	"context"
	"errors"
	"fmt"
	"reflect"

	"github.com/ava-labs/avalanchego/ids"

	"github.com/ava-labs/hypersdk/codec"
)

var (
	callInfoTypeInfo = reflect.TypeOf(CallInfo{})

	errCannotOverwrite = errors.New("trying to overwrite set field")
)

type CallContext struct {
	r               *WasmRuntime
	defaultCallInfo CallInfo
}

func (c CallContext) createCallInfo(callInfo *CallInfo) (*CallInfo, error) {
	newCallInfo := *callInfo
	resultInfo := reflect.ValueOf(&newCallInfo)
	defaults := reflect.ValueOf(c.defaultCallInfo)
	
	for i := 0; i < defaults.NumField(); i++ {
		fieldName := callInfoTypeInfo.Field(i).Name
		defaultField := defaults.Field(i)
		
		// Special handling for Actor and Fuel fields in cross-contract calls
		if fieldName == "Actor" || fieldName == "Fuel" {
			// If the field is not set in the new call info, use the default
			resultField := resultInfo.Elem().Field(i)
			if resultField.IsZero() {
				resultField.Set(defaultField)
			}
			// If the field is already set in the new call info, that's fine - we'll use that value
			continue
		}
		
		if !defaultField.IsZero() {
			resultField := resultInfo.Elem().Field(i)
			if !resultField.IsZero() {
				return nil, fmt.Errorf("%w %s", errCannotOverwrite, fieldName)
			}
			resultField.Set(defaultField)
		}
	}
	return &newCallInfo, nil
}

func (c CallContext) CallContract(ctx context.Context, info *CallInfo) ([]byte, error) {
	newInfo, err := c.createCallInfo(info)
	if err != nil {
		return nil, err
	}
	return c.r.CallContract(ctx, newInfo)
}

func (c CallContext) WithStateManager(manager StateManager) CallContext {
	c.defaultCallInfo.State = manager
	return c
}

func (c CallContext) WithActor(address codec.Address) CallContext {
	c.defaultCallInfo.Actor = address
	return c
}

func (c CallContext) WithFunction(s string) CallContext {
	c.defaultCallInfo.FunctionName = s
	return c
}

func (c CallContext) WithContract(address codec.Address) CallContext {
	c.defaultCallInfo.Contract = address
	return c
}

func (c CallContext) WithFuel(u uint64) CallContext {
	c.defaultCallInfo.Fuel = u
	return c
}

func (c CallContext) WithParams(bytes []byte) CallContext {
	c.defaultCallInfo.Params = bytes
	return c
}

func (c CallContext) WithHeight(height uint64) CallContext {
	c.defaultCallInfo.Height = height
	return c
}

func (c CallContext) WithActionID(actionID ids.ID) CallContext {
	c.defaultCallInfo.ActionID = actionID
	return c
}

func (c CallContext) WithTimestamp(timestamp uint64) CallContext {
	c.defaultCallInfo.Timestamp = timestamp
	return c
}

func (c CallContext) WithValue(value uint64) CallContext {
	c.defaultCallInfo.Value = value
	return c
}
