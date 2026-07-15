/*
 * Copyright (c) 2026 Hardik Bhaskar
 *
 * Licensed under the MIT License.
 * See the LICENSE file for details.
 */

/*
 * stb_image_impl.c — Single compilation unit that instantiates stb_image.
 *
 * We only need PNG support (MahinaReveal splash frames).  Defining
 * STBI_ONLY_PNG strips JPEG, BMP, GIF, TGA, HDR, PIC, PNM decoders,
 * keeping the compiled code under ~80 KB.
 */

#define STB_IMAGE_IMPLEMENTATION
#define STBI_ONLY_PNG
#define STBI_NO_STDIO   /* We use stbi_load() which still needs stdio via  */
#undef  STBI_NO_STDIO   /* the normal path — undo the accidental define.   */

#include "stb_image.h"
