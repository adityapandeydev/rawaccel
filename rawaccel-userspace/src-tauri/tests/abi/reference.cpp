// Standalone userspace ABI probe. Includes the project's actual custom headers.
// No kernel project, driver handle, DeviceIoControl, installation or settings writes.
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <string>
#include <type_traits>
#include <utility>
#include <vector>
#include "rawaccel-io-def.h"
#include "rawaccel.hpp"
#include "rawaccel-version.h"
namespace ra = rawaccel;
static_assert(sizeof(wchar_t) == 2 && sizeof(void*) == 8, "Run this probe for Windows x64");
using modifier_settings_data = ra::modifier_settings::data_t;
using vec2d = ::vec2d;
using accel_args = ra::accel_args;
using speed_args = ra::speed_args;
using profile = ra::profile;
using modifier_flags = ra::modifier_flags;
using accel_union = ra::accel_union;
using modifier_settings = ra::modifier_settings;
using time_clamp = ra::time_clamp;
using device_config = ra::device_config;
using device_settings = ra::device_settings;
using io_base = ra::io_base;
using version_t = ra::version_t;
using ClassicLegacy = ra::classic<false>;
using ClassicGain = ra::classic<true>;
using JumpLegacy = ra::jump<false>;
using JumpGain = ra::jump<true>;
using NaturalLegacy = ra::natural<false>;
using NaturalGain = ra::natural<true>;
using PowerLegacy = ra::power<false>;
using PowerGain = ra::power<true>;
using TieredLinearLegacy = decltype(std::declval<ra::tiered_base>().linear);
using TieredNaturalLegacy = decltype(std::declval<ra::tiered_base>().natural);
using TieredNaturalGain = decltype(std::declval<ra::tiered_base>().natural);
using Lookup = ra::lookup;
using SynchronousLegacy = ra::activation_framework<false>;
using SynchronousGain = ra::activation_framework<true>;

template<class T> void layout(const char* name, std::initializer_list<std::pair<const char*, size_t>> fields) {
    std::cout << "\"" << name << "\":{\"size\":" << sizeof(T) << ",\"align\":" << alignof(T);
    for (auto field : fields) std::cout << ",\"" << field.first << "\":" << field.second;
    std::cout << "},\n";
}
void layouts() {
    std::cout << "{\n";
    layout<vec2d>("vec2d", {{"x", offsetof(vec2d, x)}, {"y", offsetof(vec2d, y)}});
    layout<accel_args>("accel_args", {{"mode", offsetof(accel_args, mode)}, {"gain", offsetof(accel_args, gain)}, {"input_offset", offsetof(accel_args, input_offset)}, {"output_offset", offsetof(accel_args, output_offset)}, {"acceleration", offsetof(accel_args, acceleration)}, {"decay_rate", offsetof(accel_args, decay_rate)}, {"gamma", offsetof(accel_args, gamma)}, {"motivity", offsetof(accel_args, motivity)}, {"exponent_classic", offsetof(accel_args, exponent_classic)}, {"scale", offsetof(accel_args, scale)}, {"exponent_power", offsetof(accel_args, exponent_power)}, {"limit", offsetof(accel_args, limit)}, {"sync_speed", offsetof(accel_args, sync_speed)}, {"smooth", offsetof(accel_args, smooth)}, {"t_type", offsetof(accel_args, t_type)}, {"tiered_multiplier1", offsetof(accel_args, tiered_multiplier1)}, {"tiered_input_offset1", offsetof(accel_args, tiered_input_offset1)}, {"tiered_multiplier2", offsetof(accel_args, tiered_multiplier2)}, {"tiered_transition1", offsetof(accel_args, tiered_transition1)}, {"tiered_input_offset2", offsetof(accel_args, tiered_input_offset2)}, {"tiered_multiplier3", offsetof(accel_args, tiered_multiplier3)}, {"tiered_transition2", offsetof(accel_args, tiered_transition2)}, {"tiered_decay_rate1", offsetof(accel_args, tiered_decay_rate1)}, {"tiered_decay_rate2", offsetof(accel_args, tiered_decay_rate2)}, {"cap", offsetof(accel_args, cap)}, {"cap_mode", offsetof(accel_args, cap_mode)}, {"length", offsetof(accel_args, length)}, {"data", offsetof(accel_args, data)}});
    layout<speed_args>("speed_args", {{"whole", offsetof(speed_args, whole)}, {"lp_norm", offsetof(speed_args, lp_norm)}, {"input_speed_smooth_halflife", offsetof(speed_args, input_speed_smooth_halflife)}, {"scale_smooth_halflife", offsetof(speed_args, scale_smooth_halflife)}, {"output_speed_smooth_halflife", offsetof(speed_args, output_speed_smooth_halflife)}});
    layout<profile>("profile", {{"name", offsetof(profile, name)}, {"domain_weights", offsetof(profile, domain_weights)}, {"range_weights", offsetof(profile, range_weights)}, {"accel_x", offsetof(profile, accel_x)}, {"accel_y", offsetof(profile, accel_y)}, {"speed_processor_args", offsetof(profile, speed_processor_args)}, {"output_dpi", offsetof(profile, output_dpi)}, {"yx_output_dpi_ratio", offsetof(profile, yx_output_dpi_ratio)}, {"lr_output_dpi_ratio", offsetof(profile, lr_output_dpi_ratio)}, {"ud_output_dpi_ratio", offsetof(profile, ud_output_dpi_ratio)}, {"degrees_rotation", offsetof(profile, degrees_rotation)}, {"degrees_snap", offsetof(profile, degrees_snap)}, {"speed_min", offsetof(profile, speed_min)}, {"speed_max", offsetof(profile, speed_max)}});
    layout<modifier_flags>("modifier_flags", {{"apply_rotate", offsetof(modifier_flags, apply_rotate)}, {"compute_ref_angle", offsetof(modifier_flags, compute_ref_angle)}, {"apply_snap", offsetof(modifier_flags, apply_snap)}, {"clamp_speed", offsetof(modifier_flags, clamp_speed)}, {"apply_directional_weight", offsetof(modifier_flags, apply_directional_weight)}, {"apply_dir_mul_x", offsetof(modifier_flags, apply_dir_mul_x)}, {"apply_dir_mul_y", offsetof(modifier_flags, apply_dir_mul_y)}});
    layout<accel_union>("accel_union", {{"memory", 0}});
    layout<modifier_settings_data>("modifier_settings_data", {{"flags", offsetof(modifier_settings_data, flags)}, {"rot_direction", offsetof(modifier_settings_data, rot_direction)}, {"accel_x", offsetof(modifier_settings_data, accel_x)}, {"accel_y", offsetof(modifier_settings_data, accel_y)}});
    layout<modifier_settings>("modifier_settings", {{"prof", offsetof(modifier_settings, prof)}, {"data", offsetof(modifier_settings, data)}});
    layout<time_clamp>("time_clamp", {{"min", offsetof(time_clamp, min)}, {"max", offsetof(time_clamp, max)}});
    layout<device_config>("device_config", {{"disable", offsetof(device_config, disable)}, {"set_extra_info", offsetof(device_config, set_extra_info)}, {"poll_time_lock", offsetof(device_config, poll_time_lock)}, {"dpi", offsetof(device_config, dpi)}, {"polling_rate", offsetof(device_config, polling_rate)}, {"clamp", offsetof(device_config, clamp)}});
    layout<device_settings>("device_settings", {{"name", offsetof(device_settings, name)}, {"profile", offsetof(device_settings, profile)}, {"id", offsetof(device_settings, id)}, {"config", offsetof(device_settings, config)}});
    layout<io_base>("io_base", {{"default_dev_cfg", offsetof(io_base, default_dev_cfg)}, {"modifier_data_size", offsetof(io_base, modifier_data_size)}, {"device_data_size", offsetof(io_base, device_data_size)}});
    layout<version_t>("version_t", {{"major", offsetof(version_t, major)}, {"minor", offsetof(version_t, minor)}, {"patch", offsetof(version_t, patch)}});
    layout<ClassicLegacy>("ClassicLegacy", {{"accel_raised", offsetof(ClassicLegacy, accel_raised)}, {"cap", offsetof(ClassicLegacy, cap)}, {"sign", offsetof(ClassicLegacy, sign)}});
    layout<ClassicGain>("ClassicGain", {{"accel_raised", offsetof(ClassicGain, accel_raised)}, {"cap", offsetof(ClassicGain, cap)}, {"constant", offsetof(ClassicGain, constant)}, {"sign", offsetof(ClassicGain, sign)}});
    layout<JumpLegacy>("JumpLegacy", {{"step", offsetof(JumpLegacy, step)}, {"smooth_rate", offsetof(JumpLegacy, smooth_rate)}});
    layout<JumpGain>("JumpGain", {{"step", offsetof(JumpGain, step)}, {"smooth_rate", offsetof(JumpGain, smooth_rate)}, {"c", offsetof(JumpGain, C)}});
    layout<NaturalLegacy>("NaturalLegacy", {{"offset", offsetof(NaturalLegacy, offset)}, {"accel", offsetof(NaturalLegacy, accel)}, {"limit", offsetof(NaturalLegacy, limit)}});
    layout<NaturalGain>("NaturalGain", {{"offset", offsetof(NaturalGain, offset)}, {"accel", offsetof(NaturalGain, accel)}, {"limit", offsetof(NaturalGain, limit)}, {"constant", offsetof(NaturalGain, constant)}});
    layout<PowerLegacy>("PowerLegacy", {{"offset", offsetof(PowerLegacy, offset)}, {"scale", offsetof(PowerLegacy, scale)}, {"constant", offsetof(PowerLegacy, constant)}, {"cap", offsetof(PowerLegacy, cap)}});
    layout<PowerGain>("PowerGain", {{"offset", offsetof(PowerGain, offset)}, {"scale", offsetof(PowerGain, scale)}, {"constant", offsetof(PowerGain, constant)}, {"cap", offsetof(PowerGain, cap)}, {"constant_b", offsetof(PowerGain, constant_b)}});
    layout<TieredLinearLegacy>("TieredLinearLegacy", {{"m1", offsetof(TieredLinearLegacy, m1)}, {"x1", offsetof(TieredLinearLegacy, x1)}, {"m2", offsetof(TieredLinearLegacy, m2)}, {"x1_end", offsetof(TieredLinearLegacy, x1_end)}, {"inv_trans1", offsetof(TieredLinearLegacy, inv_trans1)}, {"m3", offsetof(TieredLinearLegacy, m3)}, {"x2", offsetof(TieredLinearLegacy, x2)}, {"x2_end", offsetof(TieredLinearLegacy, x2_end)}, {"inv_trans2", offsetof(TieredLinearLegacy, inv_trans2)}});
    layout<TieredNaturalLegacy>("TieredNaturalLegacy", {{"m1", offsetof(TieredNaturalLegacy, m1)}, {"x1", offsetof(TieredNaturalLegacy, x1)}, {"l1", offsetof(TieredNaturalLegacy, l1)}, {"a1", offsetof(TieredNaturalLegacy, a1)}, {"x2", offsetof(TieredNaturalLegacy, x2)}, {"v2", offsetof(TieredNaturalLegacy, v2)}, {"m2_prime", offsetof(TieredNaturalLegacy, m2_prime)}, {"l2", offsetof(TieredNaturalLegacy, l2)}, {"a2", offsetof(TieredNaturalLegacy, a2)}});
    layout<TieredNaturalGain>("TieredNaturalGain", {{"m1", offsetof(TieredNaturalGain, m1)}, {"x1", offsetof(TieredNaturalGain, x1)}, {"l1", offsetof(TieredNaturalGain, l1)}, {"a1", offsetof(TieredNaturalGain, a1)}, {"x2", offsetof(TieredNaturalGain, x2)}, {"v2", offsetof(TieredNaturalGain, v2)}, {"m2_prime", offsetof(TieredNaturalGain, m2_prime)}, {"l2", offsetof(TieredNaturalGain, l2)}, {"a2", offsetof(TieredNaturalGain, a2)}});
    layout<Lookup>("Lookup", {{"size", offsetof(Lookup, size)}, {"velocity", offsetof(Lookup, velocity)}});
    layout<SynchronousLegacy>("SynchronousLegacy", {{"log_motivity", offsetof(SynchronousLegacy, log_motivity)}, {"gamma_const", offsetof(SynchronousLegacy, gamma_const)}, {"log_syncspeed", offsetof(SynchronousLegacy, log_syncspeed)}, {"syncspeed", offsetof(SynchronousLegacy, syncspeed)}, {"sharpness", offsetof(SynchronousLegacy, sharpness)}, {"sharpness_recip", offsetof(SynchronousLegacy, sharpness_recip)}, {"use_linear_clamp", offsetof(SynchronousLegacy, use_linear_clamp)}, {"minimum_sens", offsetof(SynchronousLegacy, minimum_sens)}, {"maximum_sens", offsetof(SynchronousLegacy, maximum_sens)}});
    layout<SynchronousGain>("SynchronousGain", {{"velocity", offsetof(SynchronousGain, velocity)}, {"range_start", offsetof(SynchronousGain, range) + offsetof(ra::fp_rep_range, start)}, {"range_stop", offsetof(SynchronousGain, range) + offsetof(ra::fp_rep_range, stop)}, {"range_num", offsetof(SynchronousGain, range) + offsetof(ra::fp_rep_range, num)}, {"x_start", offsetof(SynchronousGain, x_start)}});
    layout<ra::accel_mode>("accel_mode", {});
    layout<ra::cap_mode>("cap_mode", {});
    layout<ra::tiered_type>("tiered_type", {});
    layout<bool>("cpp_bool", {});
    layout<RAWINPUTHEADER>("RAWINPUTHEADER", {{"dwType", offsetof(RAWINPUTHEADER, dwType)}, {"dwSize", offsetof(RAWINPUTHEADER, dwSize)}, {"hDevice", offsetof(RAWINPUTHEADER, hDevice)}, {"wParam", offsetof(RAWINPUTHEADER, wParam)}});
    layout<RAWMOUSE>("RAWMOUSE", {{"usFlags", offsetof(RAWMOUSE, usFlags)}, {"ulRawButtons", offsetof(RAWMOUSE, ulRawButtons)}, {"lLastX", offsetof(RAWMOUSE, lLastX)}, {"lLastY", offsetof(RAWMOUSE, lLastY)}, {"ulExtraInformation", offsetof(RAWMOUSE, ulExtraInformation)}});
    layout<RAWINPUT>("RAWINPUT", {{"header", offsetof(RAWINPUT, header)}, {"data", offsetof(RAWINPUT, data)}});
    layout<RAWINPUTDEVICELIST>("RAWINPUTDEVICELIST", {{"hDevice", offsetof(RAWINPUTDEVICELIST, hDevice)}, {"dwType", offsetof(RAWINPUTDEVICELIST, dwType)}});
    std::cout << "\"constants\":{";
    std::cout << "\"MAX_NAME_LEN\":" << ra::MAX_NAME_LEN;
    std::cout << ",\"MAX_DEV_ID_LEN\":" << ra::MAX_DEV_ID_LEN;
    std::cout << ",\"LUT_RAW_DATA_CAPACITY\":" << ra::LUT_RAW_DATA_CAPACITY;
    std::cout << ",\"READ\":" << ra::READ;
    std::cout << ",\"WRITE\":" << ra::WRITE;
    std::cout << ",\"GET_VERSION\":" << ra::GET_VERSION;
    std::cout << ",\"accel_mode.classic\":" << static_cast<int>(ra::accel_mode::classic);
    std::cout << ",\"accel_mode.jump\":" << static_cast<int>(ra::accel_mode::jump);
    std::cout << ",\"accel_mode.natural\":" << static_cast<int>(ra::accel_mode::natural);
    std::cout << ",\"accel_mode.synchronous\":" << static_cast<int>(ra::accel_mode::synchronous);
    std::cout << ",\"accel_mode.power\":" << static_cast<int>(ra::accel_mode::power);
    std::cout << ",\"accel_mode.lookup\":" << static_cast<int>(ra::accel_mode::lookup);
    std::cout << ",\"accel_mode.tiered\":" << static_cast<int>(ra::accel_mode::tiered);
    std::cout << ",\"accel_mode.noaccel\":" << static_cast<int>(ra::accel_mode::noaccel);
    std::cout << ",\"cap_mode.io\":" << static_cast<int>(ra::cap_mode::io);
    std::cout << ",\"cap_mode.in\":" << static_cast<int>(ra::cap_mode::in);
    std::cout << ",\"cap_mode.out\":" << static_cast<int>(ra::cap_mode::out);
    std::cout << ",\"tiered_type.linear\":" << static_cast<int>(ra::tiered_type::linear);
    std::cout << ",\"tiered_type.natural\":" << static_cast<int>(ra::tiered_type::natural);
    std::cout << "}}\n";
}
void fill(vec2d& value, unsigned& tag);
void fill(accel_args& value, unsigned& tag);
void fill(speed_args& value, unsigned& tag);
void fill(profile& value, unsigned& tag);
void fill(modifier_flags& value, unsigned& tag);
void fill(accel_union& value, unsigned& tag);
void fill(modifier_settings_data& value, unsigned& tag);
void fill(modifier_settings& value, unsigned& tag);
void fill(time_clamp& value, unsigned& tag);
void fill(device_config& value, unsigned& tag);
void fill(device_settings& value, unsigned& tag);
void fill(io_base& value, unsigned& tag);
void fill(version_t& value, unsigned& tag);
template<class T> void fill(T& value, unsigned& tag) {
    ++tag;
    if constexpr (std::is_same_v<T, bool>) value = tag % 2 != 0;
    else if constexpr (std::is_same_v<T, ra::accel_mode>) value = static_cast<T>(tag % 8);
    else if constexpr (std::is_same_v<T, ra::cap_mode>) value = static_cast<T>(tag % 3);
    else if constexpr (std::is_same_v<T, ra::tiered_type>) value = static_cast<T>(tag % 2);
    else if constexpr (std::is_floating_point_v<T>) value = static_cast<T>(tag) / 8;
    else value = static_cast<T>(tag);
}
template<class T, size_t N> void fill(T (&values)[N], unsigned& tag) { for (auto& v : values) fill(v, tag); }
void fill(vec2d& value, unsigned& tag) {
    fill(value.x, tag);
    fill(value.y, tag);
}
void fill(accel_args& value, unsigned& tag) {
    fill(value.mode, tag);
    fill(value.gain, tag);
    fill(value.input_offset, tag);
    fill(value.output_offset, tag);
    fill(value.acceleration, tag);
    fill(value.decay_rate, tag);
    fill(value.gamma, tag);
    fill(value.motivity, tag);
    fill(value.exponent_classic, tag);
    fill(value.scale, tag);
    fill(value.exponent_power, tag);
    fill(value.limit, tag);
    fill(value.sync_speed, tag);
    fill(value.smooth, tag);
    fill(value.t_type, tag);
    fill(value.tiered_multiplier1, tag);
    fill(value.tiered_input_offset1, tag);
    fill(value.tiered_multiplier2, tag);
    fill(value.tiered_transition1, tag);
    fill(value.tiered_input_offset2, tag);
    fill(value.tiered_multiplier3, tag);
    fill(value.tiered_transition2, tag);
    fill(value.tiered_decay_rate1, tag);
    fill(value.tiered_decay_rate2, tag);
    fill(value.cap, tag);
    fill(value.cap_mode, tag);
    fill(value.length, tag);
    fill(value.data, tag);
}
void fill(speed_args& value, unsigned& tag) {
    fill(value.whole, tag);
    fill(value.lp_norm, tag);
    fill(value.input_speed_smooth_halflife, tag);
    fill(value.scale_smooth_halflife, tag);
    fill(value.output_speed_smooth_halflife, tag);
}
void fill(profile& value, unsigned& tag) {
    fill(value.name, tag);
    fill(value.domain_weights, tag);
    fill(value.range_weights, tag);
    fill(value.accel_x, tag);
    fill(value.accel_y, tag);
    fill(value.speed_processor_args, tag);
    fill(value.output_dpi, tag);
    fill(value.yx_output_dpi_ratio, tag);
    fill(value.lr_output_dpi_ratio, tag);
    fill(value.ud_output_dpi_ratio, tag);
    fill(value.degrees_rotation, tag);
    fill(value.degrees_snap, tag);
    fill(value.speed_min, tag);
    fill(value.speed_max, tag);
}
void fill(modifier_flags& value, unsigned& tag) {
    fill(value.apply_rotate, tag);
    fill(value.compute_ref_angle, tag);
    fill(value.apply_snap, tag);
    fill(value.clamp_speed, tag);
    fill(value.apply_directional_weight, tag);
    fill(value.apply_dir_mul_x, tag);
    fill(value.apply_dir_mul_y, tag);
}
void fill(accel_union& value, unsigned& tag) {
    auto* bytes = reinterpret_cast<unsigned char*>(&value);
    for (size_t i = 0; i < sizeof(value); ++i) bytes[i] = static_cast<unsigned char>(++tag);
}
void fill(modifier_settings_data& value, unsigned& tag) {
    fill(value.flags, tag);
    fill(value.rot_direction, tag);
    fill(value.accel_x, tag);
    fill(value.accel_y, tag);
}
void fill(modifier_settings& value, unsigned& tag) {
    fill(value.prof, tag);
    fill(value.data, tag);
}
void fill(time_clamp& value, unsigned& tag) {
    fill(value.min, tag);
    fill(value.max, tag);
}
void fill(device_config& value, unsigned& tag) {
    fill(value.disable, tag);
    fill(value.set_extra_info, tag);
    fill(value.poll_time_lock, tag);
    fill(value.dpi, tag);
    fill(value.polling_rate, tag);
    fill(value.clamp, tag);
}
void fill(device_settings& value, unsigned& tag) {
    fill(value.name, tag);
    fill(value.profile, tag);
    fill(value.id, tag);
    fill(value.config, tag);
}
void fill(io_base& value, unsigned& tag) {
    fill(value.default_dev_cfg, tag);
    fill(value.modifier_data_size, tag);
    fill(value.device_data_size, tag);
}
void fill(version_t& value, unsigned& tag) {
    fill(value.major, tag);
    fill(value.minor, tag);
    fill(value.patch, tag);
}
template<class T> void append(std::vector<unsigned char>& bytes, const T& value) {
    const auto* begin = reinterpret_cast<const unsigned char*>(&value);
    bytes.insert(bytes.end(), begin, begin + sizeof(value));
}
void sentinel_packet(const std::string& path) {
    // Synthetic sentinels intentionally bypass settings validation; never send them to a driver.
    // Native objects are cleared first. Field-by-field fill leaves all padding zero.
    io_base base{}; modifier_settings modifiers[2]{}; device_settings devices[2]{};
    std::memset(&base, 0, sizeof(base)); std::memset(modifiers, 0, sizeof(modifiers)); std::memset(devices, 0, sizeof(devices));
    unsigned tag = 0;
    fill(base, tag); base.modifier_data_size = 2; base.device_data_size = 2;
    for (auto& modifier : modifiers) fill(modifier, tag);
    for (auto& device : devices) fill(device, tag);
    std::vector<unsigned char> bytes;
    append(bytes, base);
    for (const auto& modifier : modifiers) append(bytes, modifier);
    for (const auto& device : devices) append(bytes, device);
    std::ofstream stream(path);
    stream << std::hex << std::setfill('0');
    for (size_t i = 0; i < bytes.size(); ++i) { stream << std::setw(2) << unsigned(bytes[i]); if (i % 40 == 39) stream << '\n'; }
    if (bytes.size() % 40) stream << '\n';
    if (!stream) throw std::runtime_error("Cannot write packet fixture");
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
    copy_named(out.t_type, in.t_type);
    copy_named(out.tiered_multiplier1, in.tiered_multiplier1);
    copy_named(out.tiered_input_offset1, in.tiered_input_offset1);
    copy_named(out.tiered_multiplier2, in.tiered_multiplier2);
    copy_named(out.tiered_transition1, in.tiered_transition1);
    copy_named(out.tiered_input_offset2, in.tiered_input_offset2);
    copy_named(out.tiered_multiplier3, in.tiered_multiplier3);
    copy_named(out.tiered_transition2, in.tiered_transition2);
    copy_named(out.tiered_decay_rate1, in.tiered_decay_rate1);
    copy_named(out.tiered_decay_rate2, in.tiered_decay_rate2);
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
    std::memset(modifiers[1].prof.name, 0, sizeof(modifiers[1].prof.name)); wcscpy_s(modifiers[1].prof.name, L"Tiered");
    modifiers[0].prof.degrees_snap = 2;
    modifiers[0].prof.range_weights = {1, 2};
    modifiers[0].prof.speed_min = 1; modifiers[0].prof.speed_max = 40;
    modifiers[0].prof.lr_output_dpi_ratio = 1.25; modifiers[0].prof.ud_output_dpi_ratio = 0.75;
    for (auto* args : {&modifiers[1].prof.accel_x, &modifiers[1].prof.accel_y}) {
        args->mode = ra::accel_mode::tiered; args->gain = false;
        args->tiered_transition1 = 8; args->tiered_transition2 = 16; args->tiered_input_offset2 = 16;
    }
    for (auto& modifier : modifiers) ra::init_data(modifier);
    // This branch intentionally precomputes native Tiered in Rust, not its C++ constructor.
    // Exact binary fractions isolate ABI/packing from transcendental curve-math parity.
    modifiers[1].data.accel_x.tiered_l.linear = {1, 0, 1.5, 8, 0.125, 2, 16, 32, 0.0625};
    modifiers[1].data.accel_y.tiered_l.linear = modifiers[1].data.accel_x.tiered_l.linear;
    wcscpy_s(devices[0].id, L"mouseA"); wcscpy_s(devices[0].profile, L"Tiered");
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
}
int main(int argc, char** argv) {
    if (argc != 3) return 2;
    layouts(); sentinel_packet(argv[1]); settings_packet(argv[2]);
}
