#ifndef MAHINA_LGP_COMPOSITOR_MAIN_H
#define MAHINA_LGP_COMPOSITOR_MAIN_H

#include "ipc/client.h"
#include "scene/surface.h"

typedef struct {
    int epoll_fd;
    int signal_fd;
    bool running;
    uint32_t next_session_id;
    lgp_client_t *clients;
    lgp_surface_manager_t surface_manager;
} lgp_compositor_state_t;

void lgp_dispatch_pointer_motion(lgp_compositor_state_t *state, int x, int y);

#endif
