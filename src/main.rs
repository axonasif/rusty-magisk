use execute::Execute; // For simplifying external command execution.
use nix::unistd;
use std::env::set_var; // For exporting environmen vars.
use std::ffi::CString;
use std::fs; // For getting fuctions for dir creation.
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::Path; // For working with file existences.
use std::process::{exit, Command};
use sys_mount::{Mount, MountFlags, SupportedFilesystems};

pub fn bind_mount(source_file: &str, target_file: &str) {
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

pub fn executev(args: &[&str]) {
    let args: Vec<CString> = args
        .iter()
        .map(|t| CString::new(*t).expect("not a proper CString"))
        .collect();
    unistd::execv(&args[0], &args).expect("failed");
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

pub fn chmod(file: &str, mode: u32) {
    fs::set_permissions(file, fs::Permissions::from_mode(mode)).unwrap();
}

pub fn init_magisk() {
    // Overwrite init.superuser.rc
    let superuser_config = "/init.superuser.rc";
    let superuser_data = "# su daemon
service su_daemon /sbin/magisk --daemon
    seclabel u:r:su:s0
    oneshot

on property:persist.sys.root_access=0
    start su_daemon

on property:persist.sys.root_access=2
    start su_daemon

on property:persist.sys.root_access=1
    start su_daemon

on property:persist.sys.root_access=3
    start su_daemon";

    match fs::write(superuser_config, superuser_data) {
        Ok(_) => {
            chmod(superuser_config, 0o755);
        }
        Err(why) => {
            eprintln!("Error: Failed to overwrite superuser config file: {}", why);
            exit(1);
        }
    }

    // Extract magiskinit and set it up
    let magisk_bin = include_bytes!("magiskinit");
    match fs::write("/sbin/magiskinit", magisk_bin) {
        Ok(_) => {
            // Extract magisk bin from magiskinit
            chmod("/sbin/magiskinit", 0o777);
            run_externc(
                "/sbin/magiskinit",
                "-x",
                "magisk",
                "/sbin/magisk",
                "Error: Failed to extract magisk from magiskinit",
            );

            // Link magisk applets
            for file in ["su", "resetprop", "magiskhide"].iter() {
                if !Path::new(&format!("{}{}", "/sbin/", file)).exists() {
                    match symlink("/sbin/magisk", format!("{}{}", "/sbin/", file)) {
                        Ok(_) => {}
                        Err(why) => {
                            eprintln!("Error: Failed to symlink for {}: {}", file, why);
                            exit(1);
                        }
                    }
                }
            }

            for dir in [
                "/data/adb/modules",
                "/data/adb/post-fs-data.d",
                "/data/adb/services.d",
            ]
            .iter()
            {
                match fs::create_dir_all(dir) {
                    Ok(_) => {}
                    Err(_) => {
                        eprintln!("Error: Failed to create {} dir", dir);
                        exit(1);
                    }
                }
            }
        }

        Err(why) => {
            eprintln!("Error: Failed to extract magiskinit: {}", why);
            exit(1);
        }
    }
}

pub fn job() {
    // Export some possibly required environment vars
    set_var("FIRST_STAGE", "1");
    set_var("ASH_STANDALONE", "1");

    // Initialize sbin
    let bin_dir = "/sbin";

    let mirror_dir = [
        format!("{}{}", bin_dir, "/.magisk/mirror/data"),
        format!("{}{}", bin_dir, "/.magisk/mirror/system"),
    ];

    for dir in mirror_dir.iter() {
        match fs::create_dir_all(dir) {
            Ok(_) => {}
            Err(_) => {
                eprintln!("Error: Failed to create {} dir", dir);
                exit(1);
            }
        }
    }

    //// Bind data and system mirrors in /sbin
    bind_mount("/data", &mirror_dir[0]);
    bind_mount("/system", &mirror_dir[1]);

    //// Initialize magisk
    init_magisk();

    // Now let's deal with selinux if needed
    if Path::new("/sys/fs/selinux").is_dir() {
        // Fix se-context
        run_externc(
            "/system/bin/chcon",
            "u:object_r:rootfs:s0",
            "/sbin",
            "",
            "Error: Failed to change se-context of /sbin",
        );

        // Execute magiskpolicy
        if !Path::new("/sbin/magiskpolicy").exists() {
            match symlink("/sbin/magisk", "/sbin/magiskpolicy") {
                Ok(_) => {}
                Err(why) => {
                    eprintln!("Error: Failed to symlink for magiskpolicy: {}", why);
                    exit(1);
                }
            }

            run_externc(
                "/sbin/magiskpolicy",
                "--live",
                "--magisk",
                "",
                "Error: Failed to execute magiskpolicy",
            );
        }
    }

    // Swtitch process to OS init.
    let init_real = "/init.real";
    if Path::new(init_real).exists() {
        executev(&["/init.real"]);
    }
}

fn main() {
    job();
}
