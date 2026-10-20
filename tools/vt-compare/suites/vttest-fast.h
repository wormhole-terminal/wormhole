/* Included into every vttest source file (fetch-suites.sh passes it as `-include`).
 *
 * vttest pauses between a request and reading the terminal's answer (a tenth of a second, three
 * times for each report) and sleeps seconds in some tests. A terminal that is a function call away
 * answers at once, so the pauses only make the headless run slow: DECRQCRA's tests alone take half
 * a minute each. This shortens every usleep() to a quarter, which is still far longer than the
 * answer takes. The sleep(3) calls (whole seconds) are left alone.
 */
#include <unistd.h>

static inline int vttest_quick_usleep(useconds_t usec) { return usleep(usec / 4); }
#define usleep(usec) vttest_quick_usleep(usec)
