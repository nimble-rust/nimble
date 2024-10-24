/* Generated with cbindgen:0.27.0 */

#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

typedef uint64_t Handle;

/**
 * Destroys a Client instance using its handle
 */
int client_free(Handle handle);

/**
 * Creates a client instance and returns the handle
 */
Handle client_new(uint64_t now);

/**
 * Updates a Client instance with the specified absolute time
 */
int client_update(Handle handle, uint64_t now);
