/*
* Go cannot call C function pointers directly, so 
* we define wrapper C functions instead.
* 
* Moreover, this needs to be in a seperate header file since per cgo spec
* "Using //export in a file places a restriction on the preamble: ...
* it must not contain any definitions, only declarations."
* https://pkg.go.dev/cmd/cgo#hdr-C_references_to_Go
*/
#ifndef CALLBACKS_H
#define CALLBACKS_H

#include "types.h"

// Function declarations
void* allocate_and_copy(const void* data, size_t size);
Bytes copy_bytes(const void* data, size_t size);

// State management functions
BytesWithError get_value(Mutable* state, uint8_t* key, int key_len);
char* insert_value(Mutable* db, const uint8_t* key, int key_size, const uint8_t* value, int value_size);
char* remove_value(Mutable* db, const uint8_t* key, int key_size);

// Bridge functions
BytesWithError bridge_get_callback(GetStateCallback callback, void* stateObj, Bytes key);
char* bridge_insert_callback(InsertStateCallback insertFuncPtr, void *dbPtr, Bytes key, Bytes value);
char* bridge_remove_callback(RemoveStateCallback removeFuncPtr, void *dbPtr, Bytes key);

// State management
Mutable new_mutable(void* stateObj, GetStateCallback get_cb, InsertStateCallback insert_cb, RemoveStateCallback remove_cb);

#endif /* CALLBACKS_H */
