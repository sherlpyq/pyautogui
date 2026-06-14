use std::env;
use std::fs;
use std::process::Command;

const WHL_BYTES: &[u8] = include_bytes!("../../dist/pyautogui-0.9.54-cp37-abi3-win_amd64.whl");

fn main() {
    println!("Decoding embedded PyAutoGUI wheel...");
    let temp_dir = env::temp_dir().join(format!("tmp_{}", std::process::id()));
    if let Err(e) = fs::create_dir_all(&temp_dir) {
        eprintln!("Failed to create temp directory: {}", e);
        return;
    }
    let temp_whl_path = temp_dir.join("pyautogui-0.9.54-cp37-abi3-win_amd64.whl");
    if let Err(e) = fs::write(&temp_whl_path, WHL_BYTES) {
        eprintln!("Failed to write temp wheel: {}", e);
        let _ = fs::remove_dir_all(&temp_dir);
        return;
    }

    let args: Vec<String> = env::args().collect();
    let python_exe = if args.len() > 1 {
        args[1].clone()
    } else {
        "python".to_string()
    };

    println!("Installing wheel from {} using {}...", temp_whl_path.display(), python_exe);
    
    let status = Command::new(&python_exe)
        .args(&[
            "-m",
            "pip",
            "install",
            "--force-reinstall",
            &temp_whl_path.to_string_lossy(),
        ])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("PyAutoGUI optimized wheel successfully installed!");
        }
        _ if args.len() == 1 => {
            eprintln!("Failed to run pip install with 'python'. Trying 'python3'...");
            let status3 = Command::new("python3")
                .args(&[
                    "-m",
                    "pip",
                    "install",
                    "--force-reinstall",
                    &temp_whl_path.to_string_lossy(),
                ])
                .status();
            match status3 {
                Ok(s) if s.success() => {
                    println!("PyAutoGUI optimized wheel successfully installed!");
                }
                _ => {
                    eprintln!("Error: Python interpreter not found or pip command failed.");
                }
            }
        }
        _ => {
            eprintln!("Error: Specified Python interpreter not found or pip command failed.");
        }
    }

    let _ = fs::remove_dir_all(&temp_dir);
}
