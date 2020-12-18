mod utils;
use faccess::PathExt;
use libmount;
use std::{env, fs, os::unix::fs::symlink, path::Path, process::Command};
use sys_mount::{Mount, MountFlags};
use utils::{
    chmod, clone_perms, dir_is_empty, extract_file, load_modfile, remount_root, switch_init,
    wipe_old_su, KernelFsMount,
};

pub fn job() {
    // Initialize procfs
    KernelFsMount::proc();

    // Check whether we need to setup overlayFS and define bin_dir var
    remount_root();
    let bin_dir = "/sbin";
    let bin_dir: &str = {
        if Path::new("/").writable() || dir_is_empty(bin_dir) {
            if !Path::new("/").writable() {
                KernelFsMount::dev();
            }
            //// Initialize bin_dir at /sbin
            if Path::new(bin_dir).exists() {
                // When empty
                if dir_is_empty(bin_dir) {
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
                    Ok(_) => match Mount::new(bin_dir, bin_dir, "tmpfs", MountFlags::empty(), None)
                    {
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

            // Init variable value
            "/sbin"
        } else {
            // Setup devfs
            KernelFsMount::dev();

            // Load overlay kernel modules
            let mut kernel_release = match fs::read_to_string("/proc/sys/kernel/osrelease") {
                Ok(ok_result) => ok_result,
                Err(_) => {
                    switch_init();
                    String::from("")
                }
            };

            kernel_release.pop(); // Remove newline char

            for module in ["exportfs/exportfs.ko", "overlayfs/overlay.ko"].iter() {
                let mod_path = format!(
                    "/system/lib/modules/{}/kernel/fs/{}",
                    kernel_release, module
                );
                match load_modfile(&mod_path) {
                    Ok(_) => {}
                    Err(_) => {
                        println!("rusty-magisk: Failed to load overlay kernel modules");
                        switch_init();
                    }
                }
            }

            // Create overlayfs runtime dirs
            for dir in ["/dev/upper", "/dev/work"].iter() {
                match fs::create_dir_all(dir) {
                    Ok(_) => {}
                    Err(_) => {
                        println!("rusty-magisk: Failed to setup devfs for overlay");
                        switch_init();
                    }
                }
            }

            // Setup overlayfs
            if let Err(why) = clone_perms("/system/bin", "/dev/upper") {
                println!("rusty-magisk: Failed to clone perms of /system/bin into /dev/upper, trying to continue anyways: {}", why);
            }
            match libmount::Overlay::writable(
                ["/system/bin"].iter().map(|x| x.as_ref()),
                "/dev/upper",
                "/dev/work",
                "/system/bin",
            )
            .mount()
            {
                Ok(_) => {
                    wipe_old_su();
                    extract_file("/dev/chmod", include_bytes!("asset/chmod"), 777);
                    for dir in ["/system/bin"].iter() {
                        match Command::new("/dev/chmod").args(&["755", dir]).spawn() {
                            Ok(_) => if let Ok(_) = fs::remove_file("/dev/chmod") {},
                            Err(why) => {
                                println!(
                                    "rusty-magisk: Failed to chnage modes on {}: {}",
                                    dir, why
                                );
                            }
                        }
                    }
                }
                Err(_) => {
                    println!("rusty-magisk: Failed to mount overlayfs at /system/bin");
                    switch_init();
                }
            }

            // Init variable value
            "/system/bin"
        }
    };

    // Export some possibly required env vars for magisk
    env::set_var("FIRST_STAGE", "1");
    env::set_var("ASH_STANDALONE", "1");

    // Initialize vars
    let superuser_config = "/init.superuser.rc";
    let superuser_config_data: &'static [u8] = {
        if Path::new("/").writable() {
            include_bytes!("config/su")
        } else {
            include_bytes!("config/su_minimal")
        }
    };

    let magisk_config = format!("{}/{}", bin_dir, ".magisk/config");
    let magisk_apk_dir = "/system/priv-app/MagiskSu";

    let magisk_bin = format!("{}/{}", bin_dir, "magisk");
    //let magisk_su_bin = format!("{}/{}", bin_dir, "su");
    let magisk_bin_data: &'static [u8] = {
        if Path::new("/system/lib64").exists() {
            include_bytes!("asset/magisk64")
        } else {
            include_bytes!("asset/magisk")
        }
    };

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

    if Path::new("/").writable() {
        extract_file(superuser_config, superuser_config_data, 0o750);
    } else {
        extract_file("/dev/su.rc", superuser_config_data, 0o750);
        match libmount::BindMount::new("/dev/su.rc", superuser_config).mount() {
            Ok(_) => {}
            Err(_) => {
                println!("rusty-magisk: Failed to mount superuser_config");
                switch_init();
            }
        }
    }

    // Update magisk binary path
    let new_superuser_config = match fs::read_to_string(superuser_config) {
        Ok(ok_result) => ok_result,
        Err(_) => {
            println!("rusty-magisk: Failed to read new superuser_config");
            switch_init();
            String::from("")
        }
    };
    match fs::write(
        superuser_config,
        new_superuser_config.replace("magisk_bin_path", &magisk_bin),
    ) {
        Ok(_) => {}
        Err(_) => {
            println!("rusty-magisk: Failed to write new superuser_config");
            switch_init();
        }
    }

    // Extract the remaining stuff
    extract_file(&magisk_config, include_bytes!("config/magisk"), 0o755);
    extract_file(&magisk_bin, magisk_bin_data, 0o755);

    // Link magisk applets
    for file in ["su", "resetprop", "magiskhide", "magiskpolicy"].iter() {
        if !Path::new(&format!("{}/{}", bin_dir, file)).exists() {
            match symlink(&magisk_bin, format!("{}/{}", bin_dir, file)) {
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
        "/data/adb/service.d",
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
    if Path::new("/").writable() {
        if !Path::new(magisk_apk_dir).exists() {
            match fs::create_dir_all(magisk_apk_dir) {
                Ok(_) => {
                    chmod(magisk_apk_dir, 0o755);
                    extract_file(
                        &format!("{}{}", magisk_apk_dir, "/MagiskSu.apk"),
                        include_bytes!("asset/magisk.apk"),
                        0o644,
                    );
                }
                Err(why) => {
                    println!("rusty-magisk: Failed to create MagiskApkDir dir: {}", why);
                }
            }
        }
    } else {
        extract_file(
            "/data/magisk.apk",
            include_bytes!("asset/magisk.apk"),
            0o755,
        );
    }

    //// Swtitch process to OS init.
    switch_init();
}

fn main() {
    job();
}
