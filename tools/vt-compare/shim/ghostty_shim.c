/* See ghostty_shim.h. Written from libghostty-vt's public headers (include/ghostty/vt/). */
#define GHOSTTY_STATIC
#include "ghostty_shim.h"

#include <ghostty/vt.h>
#include <stdlib.h>
#include <string.h>

#ifdef __APPLE__
#include <malloc/malloc.h>
#define USABLE_SIZE(p) malloc_size(p)
#else
#include <malloc.h>
#define USABLE_SIZE(p) malloc_usable_size(p)
#endif

/* The cell size every adapter reports (CELL_PIXELS in wormhole-vt-bench). */
#define CELL_W 8
#define CELL_H 16

struct WhGhostty {
  GhosttyTerminal term;
  GhosttyAllocator allocator;
  uint16_t cols, rows;
  uint8_t *replies;
  size_t replies_len, replies_cap;
};

/* ---- A counting allocator, so allocations per MB can be measured like the Rust engines'. ---- */

static uint64_t g_allocs, g_bytes;

static void count(size_t bytes) {
  __atomic_fetch_add(&g_allocs, 1, __ATOMIC_RELAXED);
  __atomic_fetch_add(&g_bytes, bytes, __ATOMIC_RELAXED);
}

/* Alignment arrives as a power of two's exponent (Zig's std.mem.Alignment). malloc already gives
 * 16 bytes, which covers everything but page-sized requests. */
static void *a_alloc(void *ctx, size_t len, uint8_t alignment, uintptr_t ret_addr) {
  (void)ctx;
  (void)ret_addr;
  size_t align = (size_t)1 << alignment;
  void *p = NULL;
  if (align <= 16) {
    p = malloc(len ? len : 1);
  } else if (posix_memalign(&p, align, len ? len : 1) != 0) {
    p = NULL;
  }
  if (p) count(len);
  return p;
}

/* In place only when the block is already big enough, as a libc-backed allocator does. */
static bool a_resize(void *ctx, void *memory, size_t memory_len, uint8_t alignment, size_t new_len,
                     uintptr_t ret_addr) {
  (void)ctx;
  (void)memory_len;
  (void)alignment;
  (void)ret_addr;
  return new_len <= USABLE_SIZE(memory);
}

static void *a_remap(void *ctx, void *memory, size_t memory_len, uint8_t alignment, size_t new_len,
                     uintptr_t ret_addr) {
  (void)ctx;
  (void)memory_len;
  (void)ret_addr;
  if (((size_t)1 << alignment) > 16) return NULL; /* the caller allocates and copies */
  void *p = realloc(memory, new_len);
  if (p) count(new_len);
  return p;
}

static void a_free(void *ctx, void *memory, size_t memory_len, uint8_t alignment, uintptr_t ret_addr) {
  (void)ctx;
  (void)memory_len;
  (void)alignment;
  (void)ret_addr;
  free(memory);
}

static const GhosttyAllocatorVtable VTABLE = {a_alloc, a_resize, a_remap, a_free};

void wh_ghostty_alloc_counts(uint64_t *allocs, uint64_t *bytes) {
  *allocs = __atomic_load_n(&g_allocs, __ATOMIC_RELAXED);
  *bytes = __atomic_load_n(&g_bytes, __ATOMIC_RELAXED);
}

/* ---- Effects: what a host answers. ---- */

static void on_write_pty(GhosttyTerminal terminal, void *userdata, const uint8_t *data, size_t len) {
  (void)terminal;
  WhGhostty *h = userdata;
  if (h->replies_len + len > h->replies_cap) {
    size_t cap = h->replies_cap ? h->replies_cap : 256;
    while (cap < h->replies_len + len) cap *= 2;
    uint8_t *grown = realloc(h->replies, cap);
    if (!grown) return;
    h->replies = grown;
    h->replies_cap = cap;
  }
  memcpy(h->replies + h->replies_len, data, len);
  h->replies_len += len;
}

static bool on_size(GhosttyTerminal terminal, void *userdata, GhosttySizeReportSize *out) {
  (void)terminal;
  const WhGhostty *h = userdata;
  out->rows = h->rows;
  out->columns = h->cols;
  out->cell_width = CELL_W;
  out->cell_height = CELL_H;
  return true;
}

/* A VT220-class terminal with ANSI color and the clipboard, which is what the host of a terminal
 * of this kind would say; the conformance cases accept any answer. */
static bool on_device_attributes(GhosttyTerminal terminal, void *userdata, GhosttyDeviceAttributes *out) {
  (void)terminal;
  (void)userdata;
  memset(out, 0, sizeof(*out));
  out->primary.conformance_level = GHOSTTY_DA_CONFORMANCE_VT220;
  out->primary.features[0] = GHOSTTY_DA_FEATURE_ANSI_COLOR;
  out->primary.features[1] = GHOSTTY_DA_FEATURE_CLIPBOARD;
  out->primary.num_features = 2;
  out->secondary.device_type = GHOSTTY_DA_DEVICE_TYPE_VT220;
  out->secondary.firmware_version = 1;
  out->secondary.rom_cartridge = 0;
  out->tertiary.unit_id = 0;
  return true;
}

static void set_rgb(GhosttyTerminal t, GhosttyTerminalOption option, const uint8_t *rgb) {
  GhosttyColorRgb color = {rgb[0], rgb[1], rgb[2]};
  ghostty_terminal_set(t, option, &color);
}

WhGhostty *wh_ghostty_new(uint16_t cols, uint16_t rows, size_t scrollback_lines, const uint8_t *palette,
                          const uint8_t *foreground, const uint8_t *background, const uint8_t *cursor) {
  WhGhostty *h = calloc(1, sizeof(*h));
  if (!h) return NULL;
  h->allocator.ctx = NULL;
  h->allocator.vtable = &VTABLE;
  h->cols = cols;
  h->rows = rows;
  if (ghostty_terminal_new(&h->allocator, &h->term, cols, rows) != GHOSTTY_SUCCESS) {
    free(h);
    return NULL;
  }
  GhosttyTerminal t = h->term;
  ghostty_terminal_set(t, GHOSTTY_TERMINAL_OPT_USERDATA, h);
  ghostty_terminal_set(t, GHOSTTY_TERMINAL_OPT_WRITE_PTY, (const void *)on_write_pty);
  ghostty_terminal_set(t, GHOSTTY_TERMINAL_OPT_SIZE, (const void *)on_size);
  ghostty_terminal_set(t, GHOSTTY_TERMINAL_OPT_DEVICE_ATTRIBUTES, (const void *)on_device_attributes);

  GhosttyColorRgb pal[256];
  for (int i = 0; i < 256; i++) {
    pal[i].r = palette[i * 3];
    pal[i].g = palette[i * 3 + 1];
    pal[i].b = palette[i * 3 + 2];
  }
  ghostty_terminal_set(t, GHOSTTY_TERMINAL_OPT_COLOR_PALETTE, pal);
  set_rgb(t, GHOSTTY_TERMINAL_OPT_COLOR_FOREGROUND, foreground);
  set_rgb(t, GHOSTTY_TERMINAL_OPT_COLOR_BACKGROUND, background);
  set_rgb(t, GHOSTTY_TERMINAL_OPT_COLOR_CURSOR, cursor);

  /* The line limit is only an estimate (it prunes a page at a time); a zero byte limit is the one
   * way to have no history at all. */
  ghostty_terminal_set(t, GHOSTTY_TERMINAL_OPT_SCROLLBACK_MAX_LINES, &scrollback_lines);
  if (scrollback_lines == 0) {
    size_t zero = 0;
    ghostty_terminal_set(t, GHOSTTY_TERMINAL_OPT_SCROLLBACK_MAX_BYTES, &zero);
  }
  /* Sets the pixel size the size reports are made from. */
  ghostty_terminal_resize(t, cols, rows, CELL_W, CELL_H);
  return h;
}

void wh_ghostty_free(WhGhostty *h) {
  if (!h) return;
  ghostty_terminal_free(h->term);
  free(h->replies);
  free(h);
}

void wh_ghostty_feed(WhGhostty *h, const uint8_t *data, size_t len) {
  ghostty_terminal_vt_write(h->term, data, len);
}

int wh_ghostty_resize(WhGhostty *h, uint16_t cols, uint16_t rows) {
  h->cols = cols;
  h->rows = rows;
  return ghostty_terminal_resize(h->term, cols, rows, CELL_W, CELL_H) == GHOSTTY_SUCCESS ? 0 : -1;
}

const uint8_t *wh_ghostty_replies(const WhGhostty *h, size_t *len) {
  *len = h->replies_len;
  return h->replies;
}

void wh_ghostty_clear_replies(WhGhostty *h) { h->replies_len = 0; }

int wh_ghostty_cursor(const WhGhostty *h, uint16_t *col, uint16_t *row, int *visible) {
  bool shown = false;
  if (ghostty_terminal_get(h->term, GHOSTTY_TERMINAL_DATA_CURSOR_X, col) != GHOSTTY_SUCCESS) return -1;
  if (ghostty_terminal_get(h->term, GHOSTTY_TERMINAL_DATA_CURSOR_Y, row) != GHOSTTY_SUCCESS) return -1;
  if (ghostty_terminal_get(h->term, GHOSTTY_TERMINAL_DATA_CURSOR_VISIBLE, &shown) != GHOSTTY_SUCCESS) return -1;
  *visible = shown;
  return 0;
}

int wh_ghostty_alt_screen(const WhGhostty *h) {
  GhosttyTerminalScreen screen = GHOSTTY_TERMINAL_SCREEN_PRIMARY;
  ghostty_terminal_get(h->term, GHOSTTY_TERMINAL_DATA_ACTIVE_SCREEN, &screen);
  return screen == GHOSTTY_TERMINAL_SCREEN_ALTERNATE;
}

const uint8_t *wh_ghostty_title(const WhGhostty *h, size_t *len) {
  GhosttyString title = {NULL, 0};
  ghostty_terminal_get(h->term, GHOSTTY_TERMINAL_DATA_TITLE, &title);
  *len = title.len;
  return title.ptr;
}

size_t wh_ghostty_scrollback_rows(const WhGhostty *h) {
  size_t rows = 0;
  ghostty_terminal_get(h->term, GHOSTTY_TERMINAL_DATA_SCROLLBACK_ROWS, &rows);
  return rows;
}

static int grid_ref(const WhGhostty *h, int history, uint16_t x, uint32_t y, GhosttyGridRef *ref) {
  GhosttyPoint point;
  memset(&point, 0, sizeof(point));
  point.tag = history ? GHOSTTY_POINT_TAG_HISTORY : GHOSTTY_POINT_TAG_ACTIVE;
  point.value.coordinate.x = x;
  point.value.coordinate.y = y;
  *ref = GHOSTTY_INIT_SIZED(GhosttyGridRef);
  return ghostty_terminal_grid_ref(h->term, point, ref) == GHOSTTY_SUCCESS ? 0 : -1;
}

static void style_color(const GhosttyStyleColor *c, uint8_t *kind, uint8_t *rgb) {
  switch (c->tag) {
    case GHOSTTY_STYLE_COLOR_PALETTE:
      *kind = WH_COLOR_PALETTE;
      rgb[0] = c->value.palette;
      break;
    case GHOSTTY_STYLE_COLOR_RGB:
      *kind = WH_COLOR_RGB;
      rgb[0] = c->value.rgb.r;
      rgb[1] = c->value.rgb.g;
      rgb[2] = c->value.rgb.b;
      break;
    default:
      *kind = WH_COLOR_DEFAULT;
      break;
  }
}

int wh_ghostty_cell(const WhGhostty *h, int history, uint16_t x, uint32_t y, WhCell *out) {
  memset(out, 0, sizeof(*out));
  GhosttyGridRef ref;
  if (grid_ref(h, history, x, y, &ref) != 0) return -1;
  GhosttyCell cell;
  if (ghostty_grid_ref_cell(&ref, &cell) != GHOSTTY_SUCCESS) return -1;

  GhosttyCellWide wide = GHOSTTY_CELL_WIDE_NARROW;
  GhosttyCellContentTag tag = GHOSTTY_CELL_CONTENT_CODEPOINT;
  bool has_text = false;
  ghostty_cell_get(cell, GHOSTTY_CELL_DATA_WIDE, &wide);
  ghostty_cell_get(cell, GHOSTTY_CELL_DATA_CONTENT_TAG, &tag);
  ghostty_cell_get(cell, GHOSTTY_CELL_DATA_HAS_TEXT, &has_text);
  /* The second half of a wide character takes no cell of its own. The head spacer, left blank
   * at the end of a row when a wide character moved to the next, is a blank cell for a reader of
   * the screen; it is not part of the row's text. */
  out->width = wide == GHOSTTY_CELL_WIDE_WIDE ? 2 : (wide == GHOSTTY_CELL_WIDE_SPACER_TAIL ? 0 : 1);
  out->spacer_head = wide == GHOSTTY_CELL_WIDE_SPACER_HEAD;

  if (has_text) {
    size_t n = 0;
    GhosttyResult r = ghostty_grid_ref_graphemes(&ref, out->codepoints, WH_MAX_CODEPOINTS, &n);
    if (r == GHOSTTY_SUCCESS || r == GHOSTTY_OUT_OF_SPACE) {
      out->n_codepoints = n > WH_MAX_CODEPOINTS ? WH_MAX_CODEPOINTS : (uint32_t)n;
    }
  }

  GhosttyStyle style = GHOSTTY_INIT_SIZED(GhosttyStyle);
  if (ghostty_grid_ref_style(&ref, &style) == GHOSTTY_SUCCESS) {
    style_color(&style.fg_color, &out->fg_kind, out->fg);
    style_color(&style.bg_color, &out->bg_kind, out->bg);
    style_color(&style.underline_color, &out->ul_kind, out->ul);
    if (style.bold) out->attrs |= WH_ATTR_BOLD;
    if (style.faint) out->attrs |= WH_ATTR_DIM;
    if (style.italic) out->attrs |= WH_ATTR_ITALIC;
    if (style.blink) out->attrs |= WH_ATTR_BLINK;
    if (style.inverse) out->attrs |= WH_ATTR_INVERSE;
    if (style.invisible) out->attrs |= WH_ATTR_HIDDEN;
    if (style.strikethrough) out->attrs |= WH_ATTR_STRIKE;
    if (style.overline) out->attrs |= WH_ATTR_OVERLINE;
    switch (style.underline) {
      case GHOSTTY_SGR_UNDERLINE_SINGLE: out->attrs |= WH_ATTR_UNDERLINE; break;
      case GHOSTTY_SGR_UNDERLINE_DOUBLE: out->attrs |= WH_ATTR_DOUBLE_UNDERLINE; break;
      case GHOSTTY_SGR_UNDERLINE_CURLY: out->attrs |= WH_ATTR_CURLY_UNDERLINE; break;
      case GHOSTTY_SGR_UNDERLINE_DOTTED: out->attrs |= WH_ATTR_DOTTED_UNDERLINE; break;
      case GHOSTTY_SGR_UNDERLINE_DASHED: out->attrs |= WH_ATTR_DASHED_UNDERLINE; break;
      default: break;
    }
  }

  /* A cell erased with a background color holds just that color, not a style. */
  if (tag == GHOSTTY_CELL_CONTENT_BG_COLOR_PALETTE) {
    GhosttyColorPaletteIndex index = 0;
    ghostty_cell_get(cell, GHOSTTY_CELL_DATA_COLOR_PALETTE, &index);
    out->bg_kind = WH_COLOR_PALETTE;
    out->bg[0] = index;
  } else if (tag == GHOSTTY_CELL_CONTENT_BG_COLOR_RGB) {
    GhosttyColorRgb rgb = {0, 0, 0};
    ghostty_cell_get(cell, GHOSTTY_CELL_DATA_COLOR_RGB, &rgb);
    out->bg_kind = WH_COLOR_RGB;
    out->bg[0] = rgb.r;
    out->bg[1] = rgb.g;
    out->bg[2] = rgb.b;
  }
  return 0;
}

int64_t wh_ghostty_link(const WhGhostty *h, int history, uint16_t x, uint32_t y, uint8_t *buf, size_t cap) {
  GhosttyGridRef ref;
  if (grid_ref(h, history, x, y, &ref) != 0) return -1;
  size_t len = 0;
  GhosttyResult r = ghostty_grid_ref_hyperlink_uri(&ref, buf, cap, &len);
  if (r != GHOSTTY_SUCCESS && r != GHOSTTY_OUT_OF_SPACE) return -1;
  return (int64_t)len;
}

int wh_ghostty_row_wrapped(const WhGhostty *h, int history, uint32_t y) {
  GhosttyGridRef ref;
  if (grid_ref(h, history, 0, y, &ref) != 0) return -1;
  GhosttyRow row;
  if (ghostty_grid_ref_row(&ref, &row) != GHOSTTY_SUCCESS) return -1;
  bool wrapped = false;
  if (ghostty_row_get(row, GHOSTTY_ROW_DATA_WRAP, &wrapped) != GHOSTTY_SUCCESS) return -1;
  return wrapped ? 1 : 0;
}
