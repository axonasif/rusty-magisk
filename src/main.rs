mod utils;
use chrono::Local;
use env_logger::Builder;
use faccess::PathExt;
use libmount;
use log::{error, warn, LevelFilter};
use std::{env, fs, io::Write, os::unix::fs::symlink, path::Path, process::Command};
use sys_mount::{Mount, MountFlags};
use utils::{
    chmod, clone_perms, dir_is_empty, early_mode, extract_file, load_modfile, remount_root,
    switch_init, KernelFsMount,
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
                            error!("Failed to setup tmpfs at {}: {}", bin_dir, why);
                            switch_init();
                        }
                    }
                } else {
                    // When not empty
                    remount_root();
                    if !Path::new(bin_dir).writable() {
                        error!("{} is not writable", bin_dir);
                        switch_init()
                    }
                }
            } else {
                match fs::create_dir(bin_dir) {
                    Ok(_) => {
                        if let Err(why) =
                            Mount::new(bin_dir, bin_dir, "tmpfs", MountFlags::empty(), None)
                        {
                            error!("Failed to setup tmpfs at {}: {}", bin_dir, why);
                            switch_init();
                        }
                    }
                    Err(why) => {
                        error!(
                            "Root(/) is not writable, failed to initialize {}: {}",
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
                if let Err(_) = load_modfile(&mod_path) {
                    error!("Failed to load overlay kernel modules");
                    switch_init();
                }
            }

            // Create overlayfs runtime dirs
            for dir in ["/dev/upper", "/dev/work"].iter() {
                if let Err(why) = fs::create_dir_all(dir) {
                    {
                        error!("Failed to setup devfs for overlay: {}", why);
                        switch_init();
                    }
                }
            }

            // Setup overlayfs
            if let Err(why) = clone_perms("/system/bin", "/dev/upper") {
                warn!("Failed to clone perms of /system/bin into /dev/upper, trying to continue anyways: {}", why);
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
                    extract_file("/dev/chmod", include_bytes!("asset/chmod"), 777);
                    for dir in ["/system/bin"].iter() {
                        match Command::new("/dev/chmod").args(&["755", dir]).status() {
                            Ok(_) => if let Ok(_) = fs::remove_file("/dev/chmod") {},
                            Err(why) => {
                                error!("Failed to chnage modes on {}: {}", dir, why);
                            }
                        }
                    }
                }
                Err(_) => {
                    error!("Failed to mount overlayfs at /system/bin");
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

    let magisk_apk = "/data/magisk.apk";
    let magisk_apk_local = "/data/.rusty-magisk/magisk.apk";
    let magisk_config = format!("{}/{}", bin_dir, ".magisk/config");
    //let magisk_apk_dir = "/system/priv-app/MagiskSu";
    let pkgs_list = "/data/system/packages.list";
    let magisk_bin = format!("{}/{}", bin_dir, "magisk");
    //let magisk_su_bin = format!("{}/{}", bin_dir, "su");
    let magisk_bin_local = "/data/.rusty-magisk/magisk";
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
        if let Err(why) = fs::create_dir_all(dir) {
            error!("Failed to create {} dir: {}", dir, why);
        }
    }

    //// Bind data and system mirrors in bin_dir
    let mut mirror_count = 2;

    for mirror_source in ["/system", "/data"].iter() {
        if let Err(why) = Mount::new(
            mirror_source,
            &mirror_dir[mirror_count],
            "",
            MountFlags::BIND,
            None,
        ) {
            error!(
                "Failed to bind mount {} into {}: {}",
                mirror_source, &mirror_dir[mirror_count], why
            );
        }
        mirror_count -= 1;
    }

    ///////////////////////////
    //// Initialize magisk ////
    // Extract magisk and set it up
    remount_root();

    if early_mode() {
        if Path::new("/").writable() {
            extract_file(superuser_config, superuser_config_data, 0o750);
        } else {
            extract_file("/dev/su.rc", superuser_config_data, 0o750);
            if let Err(why) = libmount::BindMount::new("/dev/su.rc", superuser_config).mount() {
                error!("Failed to mount superuser_config: {}", why);
                switch_init();
            }
        }

        // Update magisk binary path
        let new_superuser_config = match fs::read_to_string(superuser_config) {
            Ok(ok_result) => ok_result,
            Err(_) => {
                error!("Failed to read new superuser_config");
                switch_init();
                String::from("")
            }
        };
        if let Err(why) = fs::write(
            superuser_config,
            new_superuser_config.replace("magisk_bin_path", &magisk_bin),
        ) {
            error!("Failed to write new superuser_config: {}", why);
            switch_init();
        }
    }

    // Extract the remaining stuff
    extract_file(&magisk_config, include_bytes!("config/magisk"), 0o755);
    if Path::new(magisk_bin_local).exists() {
        if let Err(why) = fs::copy(magisk_bin_local, &magisk_bin) {
            error!(
                "Failed to copy {} into {}: {}",
                magisk_bin_local, magisk_bin, why
            );
            switch_init()
        }
    } else {
        extract_file(&magisk_bin, magisk_bin_data, 0o755);
    }

    // Link magisk applets
    for file in ["su", "resetprop", "magiskhide", "magiskpolicy"].iter() {
        if !Path::new(&format!("{}/{}", bin_dir, file)).exists() {
            if let Err(why) = symlink(&magisk_bin, format!("{}/{}", bin_dir, file)) {
                error!("Failed to create symlink for {}: {}", file, why);
                switch_init();
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
        if let Err(why) = fs::create_dir_all(dir) {
            error!("Failed to create {} dir: {}", dir, why);
        }
    }

    // Install magiskMan into system if missing
    let pkgs_list = match fs::read_to_string(pkgs_list) {
        Ok(ans) => String::from(ans),
        Err(_) => {
            error!("Failed to read {}", pkgs_list);
            String::from("")
        }
    };
    if !String::from(pkgs_list).contains("com.topjohnwu.magisk") {
        if Path::new(magisk_apk_local).exists() {
            if let Err(why) = fs::copy(magisk_apk_local, magisk_apk) {
                error!(
                    "Failed to copy {} to {}: {}",
                    magisk_apk_local, magisk_apk, why
                );
                switch_init()
            }
        } else {
            extract_file(magisk_apk, include_bytes!("asset/magisk.apk"), 0o755);
        }
    }

    // Own pre-existing su binaries
    if bin_dir == "/sbin" {
        for su_bin in ["/system/bin/su", "/system/xbin/su"].iter() {
            if Path::new(su_bin).exists() {
                if let Err(why) =
                    libmount::BindMount::new(format!("{}/{}", bin_dir, "su"), su_bin).mount()
                {
                    error!("Failed to bind mount {} into {}: {}", bin_dir, su_bin, why);
                }
            }
        }
        // Ensure /sbin is accessible globally
        chmod(bin_dir, 0o755);
    }

    /*
    if Path::new("/sys/fs/selinux").exists() {
        extract_file("/dev/magiskpolicy", include_bytes!("asset/magiskpolicy"), 0o755);
        if let Err(_) = Command::new("/dev/magiskpolicy")
            .args(&["--live", "--magisk"])
            .status()
        {
            error!("Failed to execute magiskpolicy");
        }
    }
    */

    //// Swtitch process to OS init.
    switch_init();
}

pub fn init_logger() {
    let mut builder = Builder::new();

    builder.format(|formatter, record| {
        writeln!(
            formatter,
            "{} [{}] ({}): {}",
            Local::now().format("%Y-%m-%d %H:%M:%S"),
            record.level(),
            record.target(),
            record.args()
        )
    });

    if let Ok(var) = env::var("RUST_LOG") {
        builder.parse_filters(&var);
    } else {
        // if no RUST_LOG provided, default to logging at the Info level
        builder.filter(None, LevelFilter::Info);
        // Filter extraneous html5ever not-implemented messages
        builder.filter(Some("html5ever"), LevelFilter::Error);
    }

    builder.init();
}

fn main() {
    init_logger();
    job();
}
