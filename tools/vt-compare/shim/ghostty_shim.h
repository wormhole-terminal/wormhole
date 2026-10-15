/* A flat, plain-types face for libghostty-vt's terminal, so the Rust side declares nothing but
 * integers and pointers. libghostty-vt's own API is built from sized structs, tagged unions and
 * callbacks; compiling this file against its public headers keeps every layout right.
 *
 * Written from the public headers (include/ghostty/vt/) alone. */
#ifndef WH_GHOSTTY_SHIM_H
#define WH_GHOSTTY_SHIM_H

#include <stddef.h>
#include <stdint.h>

#define WH_MAX_CODEPOINTS 32

/* WhCell.attrs */
#define WH_ATTR_BOLD 1u
#define WH_ATTR_DIM 2u
#define WH_ATTR_ITALIC 4u
#define WH_ATTR_BLINK 8u
#define WH_ATTR_INVERSE 16u
#define WH_ATTR_HIDDEN 32u
#define WH_ATTR_STRIKE 64u
#define WH_ATTR_OVERLINE 128u
#define WH_ATTR_UNDERLINE 256u
#define WH_ATTR_DOUBLE_UNDERLINE 512u
#define WH_ATTR_CURLY_UNDERLINE 1024u
#define WH_ATTR_DOTTED_UNDERLINE 2048u
#define WH_ATTR_DASHED_UNDERLINE 4096u

/* WhCell.fg_kind and friends */
#define WH_COLOR_DEFAULT 0
#define WH_COLOR_PALETTE 1 /* the index is in the first byte of the rgb array */
#define WH_COLOR_RGB 2

typedef struct WhGhostty WhGhostty;

typedef struct {
  uint32_t codepoints[WH_MAX_CODEPOINTS];
  uint32_t n_codepoints; /* 0: the cell has no text */
  uint8_t width;         /* 1, 2, or 0 for the second half of a wide character */
  uint8_t spacer_head;   /* 1 for the blank left at the end of a row when a wide character wrapped */
  uint8_t fg_kind, bg_kind, ul_kind;
  uint8_t fg[3], bg[3], ul[3];
  uint16_t attrs;
} WhCell;

/* NULL when the terminal could not be made. `palette` is 256 RGB triples, the colors are
 * foreground, background and cursor as RGB triples. */
WhGhostty *wh_ghostty_new(uint16_t cols, uint16_t rows, size_t scrollback_lines, const uint8_t *palette,
                          const uint8_t *foreground, const uint8_t *background, const uint8_t *cursor);
void wh_ghostty_free(WhGhostty *h);
void wh_ghostty_feed(WhGhostty *h, const uint8_t *data, size_t len);
int wh_ghostty_resize(WhGhostty *h, uint16_t cols, uint16_t rows);

/* The bytes the terminal wrote to the program since wh_ghostty_clear_replies. */
const uint8_t *wh_ghostty_replies(const WhGhostty *h, size_t *len);
void wh_ghostty_clear_replies(WhGhostty *h);

/* 0 on success. */
int wh_ghostty_cursor(const WhGhostty *h, uint16_t *col, uint16_t *row, int *visible);
int wh_ghostty_alt_screen(const WhGhostty *h);
/* The title, borrowed until the next call that feeds or resizes; len 0 when none. */
const uint8_t *wh_ghostty_title(const WhGhostty *h, size_t *len);
size_t wh_ghostty_scrollback_rows(const WhGhostty *h);

/* Cell (x, y) of the screen (history = 0, y from the top row) or of the history (history = 1, y
 * from the oldest line). 0 on success. */
int wh_ghostty_cell(const WhGhostty *h, int history, uint16_t x, uint32_t y, WhCell *out);
/* The link of that cell: its length, which may exceed cap (nothing more than cap is written); 0
 * when none, -1 when the cell cannot be read. */
int64_t wh_ghostty_link(const WhGhostty *h, int history, uint16_t x, uint32_t y, uint8_t *buf, size_t cap);
/* 1 when that row continues in the next one because the text wrapped, 0 when not, -1 on error. */
int wh_ghostty_row_wrapped(const WhGhostty *h, int history, uint32_t y);

/* Calls and bytes asked of the allocator libghostty-vt was given, over the whole process. */
void wh_ghostty_alloc_counts(uint64_t *allocs, uint64_t *bytes);

#endif
