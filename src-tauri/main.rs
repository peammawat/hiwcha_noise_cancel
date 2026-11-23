use libloading::{Library, Symbol};
use std::ffi::c_int;

#[tauri::command]
fn toggle_engine(enable: bool, input_idx: i32, cable_idx: i32, threshold: f32) -> Result<String, String> {
    if enable {
        unsafe {
            let lib = Library::new("audio_engine.dll").map_err(|e| e.to_string())?;
            let start_fn: Symbol<unsafe extern "C" fn(c_int, c_int, f32) -> c_int> =
                lib.get(b"start_noise_cancel").map_err(|e| e.to_string())?;
            
            let res = start_fn(input_idx, cable_idx, threshold);
            if res == 0 {
                Ok("Noise Cancellation Started".into())
            } else {
                Err("Failed to open audio stream".into())
            }
        }
    } else {
        // Logic สำหรับหยุด Stream
        Ok("Stopped".into())
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![toggle_engine])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}