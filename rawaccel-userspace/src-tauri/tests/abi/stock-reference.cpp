// Offline stock signed-driver ABI reference, compiled against a pinned upstream release.
#include <cstddef>
#include <cstring>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <string>
#include <type_traits>
#include <vector>
#include "rawaccel-io-def.h"
#include "rawaccel.hpp"
#include "rawaccel-version.h"
namespace ra = rawaccel;
static_assert(sizeof(wchar_t) == 2 && sizeof(void*) == 8, "Windows x64 probe only");
using StockAccelArgs = ra::accel_args;
using StockProfile = ra::profile;
using StockModifier = ra::modifier_settings;
using accel_args = ra::accel_args;
using speed_args = ra::speed_args;
using profile = ra::profile;
using io_base = ra::io_base;
using modifier_settings = ra::modifier_settings;
using device_settings = ra::device_settings;
using vec2d = ::vec2d;
template<class T> void layout(const char* name, std::initializer_list<std::pair<const char*, size_t>> fields) {
    std::cout << "\"" << name << "\":{\"size\":" << sizeof(T) << ",\"align\":" << alignof(T);
    for (auto field : fields) std::cout << ",\"" << field.first << "\":" << field.second;
    std::cout << "},\n";
}
void layouts() {
    std::cout << "{\n";
    layout<StockAccelArgs>("StockAccelArgs", {{"mode", offsetof(StockAccelArgs, mode)}, {"gain", offsetof(StockAccelArgs, gain)}, {"input_offset", offsetof(StockAccelArgs, input_offset)}, {"output_offset", offsetof(StockAccelArgs, output_offset)}, {"acceleration", offsetof(StockAccelArgs, acceleration)}, {"decay_rate", offsetof(StockAccelArgs, decay_rate)}, {"gamma", offsetof(StockAccelArgs, gamma)}, {"motivity", offsetof(StockAccelArgs, motivity)}, {"exponent_classic", offsetof(StockAccelArgs, exponent_classic)}, {"scale", offsetof(StockAccelArgs, scale)}, {"exponent_power", offsetof(StockAccelArgs, exponent_power)}, {"limit", offsetof(StockAccelArgs, limit)}, {"sync_speed", offsetof(StockAccelArgs, sync_speed)}, {"smooth", offsetof(StockAccelArgs, smooth)}, {"cap", offsetof(StockAccelArgs, cap)}, {"cap_mode", offsetof(StockAccelArgs, cap_mode)}, {"length", offsetof(StockAccelArgs, length)}, {"data", offsetof(StockAccelArgs, data)}});
    layout<StockProfile>("StockProfile", {{"name", offsetof(StockProfile, name)}, {"domain_weights", offsetof(StockProfile, domain_weights)}, {"range_weights", offsetof(StockProfile, range_weights)}, {"accel_x", offsetof(StockProfile, accel_x)}, {"accel_y", offsetof(StockProfile, accel_y)}, {"speed_processor_args", offsetof(StockProfile, speed_processor_args)}, {"output_dpi", offsetof(StockProfile, output_dpi)}, {"yx_output_dpi_ratio", offsetof(StockProfile, yx_output_dpi_ratio)}, {"lr_output_dpi_ratio", offsetof(StockProfile, lr_output_dpi_ratio)}, {"ud_output_dpi_ratio", offsetof(StockProfile, ud_output_dpi_ratio)}, {"degrees_rotation", offsetof(StockProfile, degrees_rotation)}, {"degrees_snap", offsetof(StockProfile, degrees_snap)}, {"speed_min", offsetof(StockProfile, speed_min)}, {"speed_max", offsetof(StockProfile, speed_max)}});
    layout<StockModifier>("StockModifier", {{"prof", offsetof(StockModifier, prof)}, {"data", offsetof(StockModifier, data)}});
    std::cout << "\"constants\":{\"noaccel\":" << static_cast<int>(ra::accel_mode::noaccel)
        << ",\"union_size\":" << sizeof(ra::accel_union) << ",\"version\":[" << RA_VER_MAJOR << ',' << RA_VER_MINOR << ',' << RA_VER_PATCH << "]}}\n";
}
template<class T> void append(std::vector<unsigned char>& bytes, const T& value) {
    const auto* start = reinterpret_cast<const unsigned char*>(&value); bytes.insert(bytes.end(), start, start + sizeof(value));
}
// Copy named initialized defaults without copying C++ padding.
void copy_named(vec2d& out, const vec2d& in);
void copy_named(accel_args& out, const accel_args& in);
void copy_named(speed_args& out, const speed_args& in);
void copy_named(profile& out, const profile& in);
template<class T> std::enable_if_t<!std::is_array_v<T>> copy_named(T& out, const T& in) { out = in; }
template<class T, size_t N> void copy_named(T (&out)[N], const T (&in)[N]) { for (size_t i = 0; i < N; ++i) copy_named(out[i], in[i]); }
void copy_named(vec2d& out, const vec2d& in) {
    copy_named(out.x, in.x);
    copy_named(out.y, in.y);
}
void copy_named(accel_args& out, const accel_args& in) {
    copy_named(out.mode, in.mode);
    copy_named(out.gain, in.gain);
    copy_named(out.input_offset, in.input_offset);
    copy_named(out.output_offset, in.output_offset);
    copy_named(out.acceleration, in.acceleration);
    copy_named(out.decay_rate, in.decay_rate);
    copy_named(out.gamma, in.gamma);
    copy_named(out.motivity, in.motivity);
    copy_named(out.exponent_classic, in.exponent_classic);
    copy_named(out.scale, in.scale);
    copy_named(out.exponent_power, in.exponent_power);
    copy_named(out.limit, in.limit);
    copy_named(out.sync_speed, in.sync_speed);
    copy_named(out.smooth, in.smooth);
    copy_named(out.cap, in.cap);
    copy_named(out.cap_mode, in.cap_mode);
    copy_named(out.length, in.length);
    copy_named(out.data, in.data);
}
void copy_named(speed_args& out, const speed_args& in) {
    copy_named(out.whole, in.whole);
    copy_named(out.lp_norm, in.lp_norm);
    copy_named(out.input_speed_smooth_halflife, in.input_speed_smooth_halflife);
    copy_named(out.scale_smooth_halflife, in.scale_smooth_halflife);
    copy_named(out.output_speed_smooth_halflife, in.output_speed_smooth_halflife);
}
void copy_named(profile& out, const profile& in) {
    copy_named(out.name, in.name);
    copy_named(out.domain_weights, in.domain_weights);
    copy_named(out.range_weights, in.range_weights);
    copy_named(out.accel_x, in.accel_x);
    copy_named(out.accel_y, in.accel_y);
    copy_named(out.speed_processor_args, in.speed_processor_args);
    copy_named(out.output_dpi, in.output_dpi);
    copy_named(out.yx_output_dpi_ratio, in.yx_output_dpi_ratio);
    copy_named(out.lr_output_dpi_ratio, in.lr_output_dpi_ratio);
    copy_named(out.ud_output_dpi_ratio, in.ud_output_dpi_ratio);
    copy_named(out.degrees_rotation, in.degrees_rotation);
    copy_named(out.degrees_snap, in.degrees_snap);
    copy_named(out.speed_min, in.speed_min);
    copy_named(out.speed_max, in.speed_max);
}

void settings_packet(const std::string& path) {
    io_base base{}; modifier_settings modifiers[2]{}; device_settings devices[2]{};
    std::memset(&base, 0, sizeof(base)); std::memset(modifiers, 0, sizeof(modifiers)); std::memset(devices, 0, sizeof(devices));
    base.modifier_data_size = 2; base.device_data_size = 2;
    base.default_dev_cfg.dpi = 1600; base.default_dev_cfg.polling_rate = 1000;
    base.default_dev_cfg.set_extra_info = true; base.default_dev_cfg.poll_time_lock = true;
    base.default_dev_cfg.clamp.min = 1; base.default_dev_cfg.clamp.max = 8;
    const profile defaults;
    for (auto& modifier : modifiers) copy_named(modifier.prof, defaults);
    std::memset(modifiers[0].prof.name, 0, sizeof(modifiers[0].prof.name)); wcscpy_s(modifiers[0].prof.name, L"Default Profile");
    std::memset(modifiers[1].prof.name, 0, sizeof(modifiers[1].prof.name)); wcscpy_s(modifiers[1].prof.name, L"Stock Linear");
    modifiers[0].prof.degrees_snap = 2;
    modifiers[0].prof.range_weights = {1, 2};
    modifiers[0].prof.speed_min = 1; modifiers[0].prof.speed_max = 40;
    modifiers[0].prof.lr_output_dpi_ratio = 1.25; modifiers[0].prof.ud_output_dpi_ratio = 0.75;
    for (auto* args : {&modifiers[1].prof.accel_x, &modifiers[1].prof.accel_y}) { args->mode = ra::accel_mode::classic; args->gain = false; }
    for (auto& modifier : modifiers) ra::init_data(modifier);
    wcscpy_s(devices[0].id, L"mouseA"); wcscpy_s(devices[0].profile, L"Stock Linear");
    devices[0].config.dpi = 800; devices[0].config.polling_rate = 2000;
    devices[0].config.clamp.min = 1; devices[0].config.clamp.max = 8;
    wcscpy_s(devices[1].id, L"mouseB"); devices[1].config.disable = true;
    devices[1].config.clamp.min = 1; devices[1].config.clamp.max = 16;
    std::vector<unsigned char> bytes; append(bytes, base);
    for (const auto& modifier : modifiers) append(bytes, modifier);
    for (const auto& device : devices) append(bytes, device);
    std::ofstream stream(path); stream << std::hex << std::setfill('0');
    for (size_t i = 0; i < bytes.size(); ++i) { stream << std::setw(2) << unsigned(bytes[i]); if (i % 40 == 39) stream << '\n'; }
    if (bytes.size() % 40) stream << '\n';
    if (!stream) throw std::runtime_error("Cannot write settings fixture");
}int main(int argc, char** argv) { if (argc != 2) return 2; layouts(); settings_packet(argv[1]); }
