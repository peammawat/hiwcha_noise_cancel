#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![toggle_engine, list_audio_devices])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while building tauri application");
}

use libloading::{Library, Symbol};
use serde::Serialize;
use std::ffi::{c_char, c_int};
use std::sync::Mutex;

// The native callback keeps executing after `start_noise_cancel` returns, so its
// DLL must remain loaded for the lifetime of the application.
static ENGINE_LIBRARY: Mutex<Option<Library>> = Mutex::new(None);

// PortAudio host API type id for WASAPI (paWASAPI).
const PA_WASAPI: c_int = 13;

#[repr(C)]
struct PaDeviceInfo {
    struct_version: c_int,
    name: *const c_char,
    host_api: c_int,
    max_input_channels: c_int,
    max_output_channels: c_int,
    default_low_input_latency: f64,
    default_low_output_latency: f64,
    default_high_input_latency: f64,
    default_high_output_latency: f64,
    default_sample_rate: f64,
}

#[repr(C)]
struct PaHostApiInfo {
    struct_version: c_int,
    api_type: c_int,
    name: *const c_char,
    device_count: c_int,
    default_input_device: c_int,
    default_output_device: c_int,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AudioDevice {
    index: c_int,
    name: String,
    is_default: bool,
}

#[derive(Serialize)]
struct AudioDevices {
    inputs: Vec<AudioDevice>,
    outputs: Vec<AudioDevice>,
}

#[tauri::command]
fn list_audio_devices() -> Result<AudioDevices, String> {
    unsafe {
        let portaudio = Library::new("libportaudio.dll")
            .map_err(|error| format!("Could not load PortAudio: {error}"))?;
        let initialize: Symbol<unsafe extern "C" fn() -> c_int> =
            portaudio.get(b"Pa_Initialize").map_err(|e| e.to_string())?;
        let terminate: Symbol<unsafe extern "C" fn() -> c_int> =
            portaudio.get(b"Pa_Terminate").map_err(|e| e.to_string())?;
        let count: Symbol<unsafe extern "C" fn() -> c_int> = portaudio
            .get(b"Pa_GetDeviceCount")
            .map_err(|e| e.to_string())?;
        let get_info: Symbol<unsafe extern "C" fn(c_int) -> *const PaDeviceInfo> = portaudio
            .get(b"Pa_GetDeviceInfo")
            .map_err(|e| e.to_string())?;
        let host_api_count: Symbol<unsafe extern "C" fn() -> c_int> = portaudio
            .get(b"Pa_GetHostApiCount")
            .map_err(|e| e.to_string())?;
        let get_default_host_api: Symbol<unsafe extern "C" fn() -> c_int> = portaudio
            .get(b"Pa_GetDefaultHostApi")
            .map_err(|e| e.to_string())?;
        let get_host_api_info: Symbol<unsafe extern "C" fn(c_int) -> *const PaHostApiInfo> =
            portaudio
                .get(b"Pa_GetHostApiInfo")
                .map_err(|e| e.to_string())?;

        let status = initialize();
        if status != 0 {
            return Err(format!("PortAudio initialization failed ({status})."));
        }

        // PortAudio lists each physical device once per host API (MME, DirectSound,
        // WASAPI, WDM-KS...). Use only one host API so every device appears once.
        // Prefer WASAPI; otherwise fall back to PortAudio's default host API.
        let result = (|| -> Result<AudioDevices, String> {
            let api_total = host_api_count();
            if api_total < 0 {
                return Err(format!("PortAudio could not list host APIs ({api_total})."));
            }
            let mut chosen: Option<c_int> = None;
            for api in 0..api_total {
                let info = get_host_api_info(api);
                if !info.is_null() && (*info).api_type == PA_WASAPI {
                    chosen = Some(api);
                    break;
                }
            }
            let api = match chosen {
                Some(api) => api,
                None => {
                    let default_api = get_default_host_api();
                    if default_api < 0 {
                        return Err(format!("PortAudio has no host API ({default_api})."));
                    }
                    default_api
                }
            };
            let api_info = get_host_api_info(api);
            if api_info.is_null() {
                return Err("PortAudio could not read the host API information.".into());
            }
            let api_info = &*api_info;

            let total = count();
            if total < 0 {
                return Err(format!("PortAudio could not list devices ({total})."));
            }

            let mut devices = AudioDevices {
                inputs: Vec::new(),
                outputs: Vec::new(),
            };
            for index in 0..total {
                let info = get_info(index);
                if info.is_null() {
                    continue;
                }
                let info = &*info;
                if info.host_api != api {
                    continue;
                }
                let name = if info.name.is_null() {
                    "Unknown device".to_string()
                } else {
                    std::ffi::CStr::from_ptr(info.name)
                        .to_string_lossy()
                        .into_owned()
                };
                if info.max_input_channels > 0 {
                    devices.inputs.push(AudioDevice {
                        index,
                        name: name.clone(),
                        is_default: index == api_info.default_input_device,
                    });
                }
                if info.max_output_channels > 0 {
                    devices.outputs.push(AudioDevice {
                        index,
                        name,
                        is_default: index == api_info.default_output_device,
                    });
                }
            }
            Ok(devices)
        })();

        terminate();
        result
    }
}

#[tauri::command]
fn toggle_engine(
    enable: bool,
    input_idx: i32,
    cable_idx: i32,
    threshold: f32,
) -> Result<String, String> {
    if !enable {
        return Err("The current audio engine can be stopped by closing the application. A live stop command is not exported by audio_engine.dll yet.".into());
    }

    let mut engine_library = ENGINE_LIBRARY
        .lock()
        .map_err(|_| "Audio engine state is unavailable".to_string())?;

    if engine_library.is_some() {
        return Err("Noise cancellation is already running.".into());
    }

    unsafe {
        let library = Library::new("audio_engine.dll")
            .map_err(|error| format!("Could not load audio_engine.dll: {error}"))?;
        let start: Symbol<unsafe extern "C" fn(c_int, c_int, f32) -> c_int> = library
            .get(b"start_noise_cancel")
            .map_err(|error| format!("Could not find start_noise_cancel: {error}"))?;

        if start(input_idx, cable_idx, threshold) != 0 {
            return Err("The audio stream could not be opened. Check the selected device indexes and VB-Audio Cable.".into());
        }

        *engine_library = Some(library);
    }

    Ok("Noise cancellation started.".into())
}
