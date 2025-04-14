#include "callbacks.h"
#include <stdlib.h>
#include <string.h>

// Static callback variables
GetStateCallback get_cb = NULL;
InsertStateCallback insert_cb = NULL;
RemoveStateCallback remove_cb = NULL;

void* allocate_and_copy(const void* data, size_t size) {
    if (data == NULL || size == 0) {
        return NULL;
    }
    void* ptr = malloc(size);
    if (ptr != NULL) {
        memcpy(ptr, data, size);
    }
    return ptr;
}

Bytes copy_bytes(const void* data, size_t size) {
    Bytes result = {0};
    if (data != NULL && size > 0) {
        void* copy = allocate_and_copy(data, size);
        if (copy != NULL) {
            result.data = (const uint8_t*)copy;
            result.length = size;
        }
    }
    return result;
}

BytesWithError get_value(Mutable* state, uint8_t* key, int key_len) {
    BytesWithError result = {0};
    
    if (!state || (key == NULL && key_len != 0) || (key != NULL && key_len <= 0)) {
        result.error = strdup("invalid arguments");
        return result;
    }

    // Only copy key if it's not null
    uint8_t* key_copy = NULL;
    if (key != NULL && key_len > 0) {
        key_copy = (uint8_t*)malloc(key_len);
        if (!key_copy) {
            result.error = strdup("failed to allocate memory for key");
            return result;
        }
        memcpy(key_copy, key, key_len);
    }

    Bytes key_bytes = {
        .data = key_copy,
        .length = key_len
    };

    // Call the callback
    result = bridge_get_callback(state->get_value_callback, state->stateObj, key_bytes);

    // Free the key copy
    if (key_copy) {
        free(key_copy);
    }

    return result;
}

char* insert_value(Mutable* db, const uint8_t* key, int key_size, const uint8_t* value, int value_size) {
    if (!db || !key || key_size <= 0 || !value || value_size <= 0) {
        return strdup("invalid arguments");
    }

    // Copy key and value
    uint8_t* key_copy = (uint8_t*)malloc(key_size);
    if (!key_copy) {
        return strdup("failed to allocate memory for key");
    }
    memcpy(key_copy, key, key_size);

    uint8_t* value_copy = (uint8_t*)malloc(value_size);
    if (!value_copy) {
        free(key_copy);
        return strdup("failed to allocate memory for value");
    }
    memcpy(value_copy, value, value_size);

    Bytes key_bytes = {
        .data = key_copy,
        .length = key_size
    };

    Bytes value_bytes = {
        .data = value_copy,
        .length = value_size
    };

    // Call the callback
    char* error = bridge_insert_callback(db->insert_callback, db->stateObj, key_bytes, value_bytes);

    // Free copies
    free(key_copy);
    free(value_copy);

    return error;
}

char* remove_value(Mutable* db, const uint8_t* key, int key_size) {
    if (!db || !key || key_size <= 0) {
        return strdup("invalid arguments");
    }

    // Copy key
    uint8_t* key_copy = (uint8_t*)malloc(key_size);
    if (!key_copy) {
        return strdup("failed to allocate memory for key");
    }
    memcpy(key_copy, key, key_size);

    Bytes key_bytes = {
        .data = key_copy,
        .length = key_size
    };

    // Call the callback
    char* error = bridge_remove_callback(db->remove_callback, db->stateObj, key_bytes);

    // Free copy
    free(key_copy);

    return error;
}

BytesWithError bridge_get_callback(GetStateCallback callback, void* stateObj, Bytes key) {
    if (!callback) {
        BytesWithError result = {0};
        result.error = strdup("null callback");
        return result;
    }
    return callback(stateObj, key);
}

char* bridge_insert_callback(InsertStateCallback insertFuncPtr, void *dbPtr, Bytes key, Bytes value) {
    if (!insertFuncPtr) {
        return strdup("null callback");
    }
    return insertFuncPtr(dbPtr, key, value);
}

char* bridge_remove_callback(RemoveStateCallback removeFuncPtr, void *dbPtr, Bytes key) {
    if (!removeFuncPtr) {
        return strdup("null callback");
    }
    return removeFuncPtr(dbPtr, key);
}

Mutable new_mutable(void* stateObj, GetStateCallback get_cb_fn, InsertStateCallback insert_cb_fn, RemoveStateCallback remove_cb_fn) {
    Mutable mutable = {
        .stateObj = stateObj,
        .get_value_callback = get_cb_fn,
        .insert_callback = insert_cb_fn,
        .remove_callback = remove_cb_fn
    };
    return mutable;
}
