/*
 *
 * This header file defines a common set of types and structures used as part of the
 * Foreign Function Interface between Go(CGO) in the context
 * of a smart contract simulator.
 *
 */

#ifndef TYPES_H
#define TYPES_H

#include <stdint.h>
#include <stdlib.h>

// Basic types
typedef struct {
    const uint8_t* data;
    size_t length;
} Bytes;

typedef struct {
    Bytes bytes;
    char* error;  // Caller must free this if non-null
} BytesWithError;

typedef Bytes ContractId;

// Address type for contracts and actors
typedef struct {
    unsigned char address[33];
} Address;

// Context needed to invoke a contract's method
typedef struct {
    // address of the contract being invoked
    Address contract_address;
    // invoker
    Address actor_address;
    // block height
    uint64_t height;
    // block timestamp
    uint64_t timestamp;
    // method being called on contract
    const char* method;
    // params borsh serialized as byte vector
    Bytes params;
    // max allowed gas during execution
    uint64_t max_gas;
} SimulatorCallContext;

// Response from calling a contract
typedef struct {
    char* error;
    Bytes result;
    uint64_t fuel;
} CallContractResponse;

// Response from creating a contract
typedef struct {
    char* error;
    ContractId contract_id;
    Address contract_address;
} CreateContractResponse;

// Callback function types
typedef BytesWithError (*GetStateCallback)(void* data, Bytes key);
typedef char* (*InsertStateCallback)(void* data, Bytes key, Bytes value);
typedef char* (*RemoveStateCallback)(void* data, Bytes key);

// State management
typedef struct {
    void* stateObj;
    GetStateCallback get_value_callback;
    InsertStateCallback insert_callback;
    RemoveStateCallback remove_callback;
} Mutable;

#endif /* TYPES_H */
