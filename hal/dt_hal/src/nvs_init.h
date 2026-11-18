/* One-time NVS flash initialisation, shared by Wi-Fi and hal_store. */
#ifndef DT_HAL_NVS_INIT_H
#define DT_HAL_NVS_INIT_H

#include "hal_err.h"

/* Initialise NVS once. If the partition is full or from a newer format, it
 * is erased and re-initialised (losing saved call counts, never bricking). */
hal_err_t hal_nvs_init(void);

#endif
