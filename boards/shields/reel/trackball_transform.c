#include <limits.h>

#include <zephyr/device.h>
#include <zephyr/devicetree.h>
#include <zephyr/input/input.h>
#include <zephyr/sys/util.h>

#include <zmk/endpoints.h>
#include <zmk/hid.h>

#define TRACKBALL_NODE DT_NODELABEL(trackball)

static int32_t raw_x;
static int32_t raw_y;
static int32_t horizontal_remainder;

static int16_t clamp_to_int16(int32_t value) {
    return (int16_t)CLAMP(value, INT16_MIN, INT16_MAX);
}

static void reel_trackball_input_handler(struct input_event *event, void *user_data) {
    ARG_UNUSED(user_data);

    if (event->type != INPUT_EV_REL) {
        return;
    }

    switch (event->code) {
    case INPUT_REL_X:
        raw_x += event->value;
        break;
    case INPUT_REL_Y:
        raw_y += event->value;
        break;
    default:
        return;
    }

    if (!event->sync) {
        return;
    }

    /*
     * The sensor is mounted at an angle to the intended ball motion.
     * Measured cursor motion is right=(+x,+y), left=(-x,-y), while the
     * vertical direction is inverted. Correct that basis with:
     *
     *     output_x = raw_y
     *     output_y = output_x - raw_x
     *
     * Horizontal response remains at the previously selected 80%.
     */
    int32_t scaled_x_numerator = raw_y * 4 + horizontal_remainder;
    int32_t output_x = scaled_x_numerator / 5;
    int32_t output_y = output_x - raw_x;

    horizontal_remainder = scaled_x_numerator - output_x * 5;

    zmk_hid_mouse_movement_set(clamp_to_int16(output_x), clamp_to_int16(output_y));
    zmk_endpoint_send_mouse_report();
    zmk_hid_mouse_movement_set(0, 0);

    raw_x = 0;
    raw_y = 0;
}

INPUT_CALLBACK_DEFINE(DEVICE_DT_GET(TRACKBALL_NODE), reel_trackball_input_handler, NULL);
