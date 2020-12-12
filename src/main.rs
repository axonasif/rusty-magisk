use std::env::set_var;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use sys_mount::{unmount, Mount, MountFlags, UnmountFlags};
mod utils;
use utils::{extract_file, remount_root, switch_init};

pub fn job() {
    // Export some possibly required env vars for magisk
    set_var("FIRST_STAGE", "1");
    set_var("ASH_STANDALONE", "1");

    // Initialize vars
    let bin_dir = "/sbin";

    let superuser_config = "/init.superuser.rc";
    let magisk_config = &format!("{}{}", bin_dir, "/.magisk/config");

    let magisk_apk_dir = "/system/priv-app/MagiskSu";
    let magisk_bin = &format!("{}{}", bin_dir, "/magisk");

    let _magisk_bin_data_x86 = include_bytes!("asset/magisk");
    let _magisk_bin_data_x64 = include_bytes!("asset/magisk64");
    let magisk_bin_data: &'static [u8] = if Path::new("/system/lib64").exists() {
        _magisk_bin_data_x64
    } else {
        _magisk_bin_data_x86
    };

    //// Initialize bin_dir
    if Path::new(bin_dir).exists() {
        // When empty
        if PathBuf::from(bin_dir)
            .read_dir()
            .map(|mut i| i.next().is_none())
            .unwrap_or(false)
        {
            match Mount::new(bin_dir, bin_dir, "tmpfs", MountFlags::empty(), None) {
                Ok(_) => {}
                Err(why) => {
                    println!(
                        "rusty-magisk: Failed to setup tmpfs at {}: {}",
                        bin_dir, why
                    );
                    switch_init();
                }
            }
        } else {
            // When not empty
            remount_root();
            match fs::write(format!("{}/{}", bin_dir, ".rwfs"), "") {
                Ok(_) => match fs::remove_file(format!("{}/{}", bin_dir, ".rwfs")) {
                    Ok(_) => {}
                    Err(_) => {}
                },
                Err(why) => {
                    println!("rusty-magisk: {} is not writable: {}", bin_dir, why);
                    switch_init();
                }
            }
        }
    } else {
        match fs::create_dir(bin_dir) {
            Ok(_) => match Mount::new(bin_dir, bin_dir, "tmpfs", MountFlags::empty(), None) {
                Ok(_) => {}
                Err(why) => {
                    println!(
                        "rusty-magisk: Failed to setup tmpfs at {}: {}",
                        bin_dir, why
                    );
                    switch_init();
                }
            },
            Err(why) => {
                println!(
                    "rusty-magisk: Root(/) is not writable, failed to initialize {}: {}",
                    bin_dir, why
                );
                switch_init();
            }
        }
    }

    // Initialize procfs
    if !Path::new("/proc/cpuinfo").exists() {
        match Mount::new("/proc", "/proc", "proc", MountFlags::empty(), None) {
            Ok(_) => {}
            Err(_) => {
                println!("rusty-magisk: Failed to initialize procfs");
                switch_init();
            }
        }
    }

    // Create required dirs in bin_dir
    let mirror_dir = [
        format!("{}{}", bin_dir, "/.magisk/modules"),
        format!("{}{}", bin_dir, "/.magisk/mirror/data"),
        format!("{}{}", bin_dir, "/.magisk/mirror/system"),
    ];

    for dir in mirror_dir.iter() {
        match fs::create_dir_all(dir) {
            Ok(_) => {}
            Err(why) => {
                println!("rusty-magisk: Failed to create {} dir: {}", dir, why);
            }
        }
    }

    //// Bind data and system mirrors in bin_dir
    let mut mirror_count = 2;

    for mirror_source in ["/system", "/data"].iter() {
        match Mount::new(
            mirror_source,
            &mirror_dir[mirror_count],
            "",
            MountFlags::BIND,
            None,
        ) {
            Ok(_) => {}
            Err(why) => {
                eprintln!(
                    "rusty-magisk: Failed to bind mount {} into {}: {}",
                    mirror_source, &mirror_dir[mirror_count], why
                );
            }
        }
        mirror_count -= 1;
    }

    ///////////////////////////
    //// Initialize magisk ////
    // Extract magisk and set it up
    remount_root();
    extract_file(superuser_config, include_bytes!("config/su"), 0o755);
    extract_file(magisk_config, include_bytes!("config/magisk"), 0o755);
    extract_file(magisk_bin, magisk_bin_data, 0o755);

    // Link magisk applets
    for file in ["su", "resetprop", "magiskhide"].iter() {
        if !Path::new(&format!("{}/{}", bin_dir, file)).exists() {
            match symlink(magisk_bin, format!("{}/{}", bin_dir, file)) {
                Ok(_) => {}
                Err(why) => {
                    println!(
                        "rusty-magisk: Failed to create symlink for {}: {}",
                        file, why
                    );
                    switch_init();
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
            Err(why) => {
                println!("rusty-magisk: Failed to create {} dir: {}", dir, why);
            }
        }
    }

    // Install magiskMan into system if missing
    if !Path::new(magisk_apk_dir).exists() {
        match fs::create_dir_all(magisk_apk_dir) {
            Ok(_) => extract_file(
                &format!("{}{}", magisk_apk_dir, "/MagiskSu.apk"),
                include_bytes!("asset/magisk.apk"),
                0o644,
            ),
            Err(why) => {
                println!("rusty-magisk: Failed to create MagiskApkDir dir: {}", why);
            }
        }
    }

    //// Swtitch process to OS init.
    // Unmount our /proc to ensure real android init doesn't panic
    match unmount("/proc", UnmountFlags::DETACH) {
        Ok(_) => {}
        Err(why) => {
            println!(
                "rusty-magisk: Failed to detach /proc, trying to switch init anyway: {}",
                why
            );
        }
    }
    switch_init();
}

fn main() {
    job();
}
