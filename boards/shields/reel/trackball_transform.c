#include <limits.h>

#include <zephyr/device.h>
#include <zephyr/devicetree.h>
#include <zephyr/input/input.h>
#include <zephyr/sys/util.h>

#include <drivers/input_processor.h>
#include <zmk/endpoints.h>
#include <zmk/hid.h>

#define TRACKBALL_NODE DT_NODELABEL(trackball)
#define AUTO_MOUSE_LAYER_NODE DT_NODELABEL(auto_mouse_layer)

#define TRACKBALL_MATRIX_SCALE 1000000
#define TRACKBALL_M00 79840
#define TRACKBALL_M01 812451
#define TRACKBALL_M10 -975784
#define TRACKBALL_M11 90474

#define MOUSE_LAYER 3
#define AUTO_MOUSE_LAYER_TIMEOUT_MS 5000

static int32_t raw_x;
static int32_t raw_y;
static int64_t output_x_remainder;
static int64_t output_y_remainder;
static struct zmk_input_processor_state auto_mouse_layer_state;

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

    int64_t output_x_numerator =
        (int64_t)raw_x * TRACKBALL_M00 +
        (int64_t)raw_y * TRACKBALL_M01 +
        output_x_remainder;
    int64_t output_y_numerator =
        (int64_t)raw_x * TRACKBALL_M10 +
        (int64_t)raw_y * TRACKBALL_M11 +
        output_y_remainder;

    int32_t output_x = (int32_t)(output_x_numerator / TRACKBALL_MATRIX_SCALE);
    int32_t output_y = (int32_t)(output_y_numerator / TRACKBALL_MATRIX_SCALE);

    output_x_remainder =
        output_x_numerator - (int64_t)output_x * TRACKBALL_MATRIX_SCALE;
    output_y_remainder =
        output_y_numerator - (int64_t)output_y * TRACKBALL_MATRIX_SCALE;

    if (raw_x != 0 || raw_y != 0) {
        zmk_input_processor_handle_event(
            DEVICE_DT_GET(AUTO_MOUSE_LAYER_NODE), event, MOUSE_LAYER,
            AUTO_MOUSE_LAYER_TIMEOUT_MS, &auto_mouse_layer_state);
    }

    zmk_hid_mouse_movement_set(clamp_to_int16(output_x), clamp_to_int16(output_y));
    zmk_endpoint_send_mouse_report();
    zmk_hid_mouse_movement_set(0, 0);

    raw_x = 0;
    raw_y = 0;
}

INPUT_CALLBACK_DEFINE(DEVICE_DT_GET(TRACKBALL_NODE), reel_trackball_input_handler, NULL);
