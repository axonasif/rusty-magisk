use nix::unistd;
use std::ffi::CString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{exit, Command}; // For working with files

/*
use sys_mount::{Mount, MountFlags, SupportedFilesystems};
use execute::Execute; // For simplifying external command execution.
pub fn mount(source_file: &str, target_file: &str) {
    // Fetch a list of supported file systems.
    // When mounting, a file system will be selected from this.
    let supported = SupportedFilesystems::new().unwrap();

    // Attempt to mount the src device to the dest directory.
    let mount_result = Mount::new(source_file, target_file, &supported, MountFlags::BIND, None);

    match mount_result {
        Ok(_mount) => {
            // Make the mount temporary, so that it will be unmounted on drop.
            // let mount = _mount.into_unmount_drop(UnmountFlags::DETACH);
        }
        Err(why) => {
            eprintln!("Error: Failed to mount device: {}", why);
            exit(1);
        }
    }
}

pub fn run_externc(
    executable: &str,
    exe_arg1: &str,
    exe_arg2: &str,
    exe_arg3: &str,
    err_msg: &str,
) {
    let mut command = Command::new(executable);

    // Noob way XD
    command.arg(exe_arg1);
    command.arg(exe_arg2);
    if exe_arg3 != "" {
        command.arg(exe_arg3);
    }

    if let Some(exit_code) = command.execute().unwrap() {
        if exit_code != 0 {
            eprintln!("{}", err_msg);
            exit(exit_code);
        }
    }
}
*/

pub fn executev(args: &[&str]) {
    let args: Vec<CString> = args
        .iter()
        .map(|t| CString::new(*t).expect("not a proper CString"))
        .collect();
    unistd::execv(&args[0], &args).expect("failed");
}

pub fn chmod(file: &str, mode: u32) {
    fs::set_permissions(file, fs::Permissions::from_mode(mode)).unwrap();
}

pub fn extract_file(extern_file: &str, intern_file: &'static [u8], extern_mode: u32) {
    match fs::write(extern_file, intern_file) {
        Ok(_) => {
            chmod(extern_file, extern_mode);
        }
        Err(why) => {
            eprintln!("Error: Failed to write {} file: {}", extern_file, why);
            exit(1);
        }
    }
}

pub fn mount(my_args: &[&str]) {
    let mount_prog = "/dev/mount";

    if !Path::new(mount_prog).exists() {
        extract_file(mount_prog, include_bytes!("asset/mount"), 0o777)
    }

    Command::new(mount_prog)
        .args(my_args)
        .spawn()
        .expect("Error: ");
}
