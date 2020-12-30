use likemod::errors;
use std::{
    fs,
    os::unix::{fs::PermissionsExt, process::CommandExt},
    path::{Path, PathBuf},
    process, thread, time,
};
use sys_mount::{unmount, Mount, MountFlags, UnmountFlags};

pub struct KernelFsMount();
impl KernelFsMount {
    pub fn proc() {
        if !Path::new("/proc/cpuinfo").exists() && early_mode() {
            match Mount::new("/proc", "/proc", "proc", MountFlags::empty(), None) {
                Ok(_) => {}
                Err(why) => {
                    println!("rusty-magisk: Failed to initialize procfs: {}", why);
                    switch_init();
                }
            }
        }
    }

    pub fn dev() {
        if dir_is_empty("/dev") && early_mode() {
            match Mount::new("/dev", "/dev", "tmpfs", MountFlags::empty(), None) {
                Ok(_) => {}
                Err(why) => {
                    println!("rusty-magisk: Failed to setup devfs for overlay: {}", why);
                    switch_init();
                }
            }
        }
    }
}

pub fn chmod(file: &str, mode: u32) {
    match fs::set_permissions(file, fs::Permissions::from_mode(mode)) {
        Ok(_) => {}
        Err(why) => {
            println!(
                "rusty-magisk: Failed to chnage file mode to {} for {}: {}",
                file, mode, why
            );
            switch_init();
        }
    }
}

pub fn extract_file(extern_file: &str, intern_file: &'static [u8], extern_mode: u32) {
    match fs::write(extern_file, intern_file) {
        Ok(_) => {
            chmod(extern_file, extern_mode);
        }
        Err(why) => {
            println!(
                "rusty-magisk: Failed to write {} file: {}",
                extern_file, why
            );
            switch_init();
        }
    }
}

pub fn switch_init() {
    if early_mode() {
        let init_real = "/init.real";
        if Path::new(init_real).exists() {
            // Unmount our /proc and /dev to ensure real android init doesn't panic
            for fs in ["/dev", "/proc"].iter() {
                // Verify fs in not empty before unmounting
                if !dir_is_empty(fs) {
                    match unmount(fs, UnmountFlags::DETACH) {
                        Ok(_) => {}
                        Err(why) => {
                            println!(
                            "rusty-magisk: Failed to detach {}, trying to switch init anyway: {}",
                            fs, why
                        );
                        }
                    }
                }
            }
            process::Command::new(init_real).exec();
        } else {
            println!(
                "rusty-magisk: No init executable found to switch to ... im gonna panniccccc!!!"
            );
            thread::sleep(time::Duration::from_secs(5));
            panic!("Once upon a time there lived ...");
        }
    } else {
        process::exit(0);
    }
}

pub fn remount_root() {
    if let Ok(_) = Mount::new("/", "/", "", MountFlags::REMOUNT, None) {}
}

pub fn dir_is_empty(dir: &str) -> bool {
    if Path::new(dir).exists()
        && PathBuf::from(dir)
            .read_dir()
            .map(|mut i| i.next().is_none())
            .unwrap_or(false)
    {
        true
    } else {
        false
    }
}

pub fn load_modfile(modpath: &str) -> errors::Result<()> {
    // Get a file descriptor to the kernel module object.
    let fmod = std::fs::File::open(Path::new(modpath))?;

    // Assemble module parameters for loading.
    let mut params = likemod::ModParams::new();
    params.insert("bus_delay".to_string(), likemod::ModParamValue::Int(5));

    // Try to load the module.
    let loader = likemod::ModLoader::default().set_parameters(params);
    loader.load_module_file(&fmod)
}

pub fn clone_perms(source: &str, target: &str) -> std::io::Result<()> {
    let perms = fs::metadata(source)?.permissions();
    fs::set_permissions(target, perms)?;
    Ok(())
}

pub fn wipe_old_su() {
    for su_bin in ["/system/bin/su", "/system/xbin/su"].iter() {
        if Path::new(su_bin).exists() {
            match fs::remove_file(su_bin) {
                Ok(_) => {}

                Err(why) => {
                    println!(
                        "rusty-magisk: Failed to remove existing {} binary: {}",
                        su_bin, why
                    );
                }
            }
        }

        /*
        match symlink("/sbin/su", su_bin) {
            Ok(_) => {}
            Err(why) => {
                println!("rusty-magisk: Failed to symlink for {}: {}", su_bin, why);
            }
        }
        */
    }
}

pub fn early_mode() -> bool {
    if Path::new("/android").exists() {
        true
    } else {
        false
    }
}

////// Some unused rusty functions below

/*

pub fn sbin_mode() -> bool {
    if env::var("ANDROID_BOOTLOGO").is_err() {
        true
    } else {
        false
    }
}


*/

/* My noobish deprecieated mount function
pub fn mount(my_args: &[&str]) {
    let mount_prog = "/dev/mount_bin";

    if !Path::new(mount_prog).exists() {
        extract_file(mount_prog, include_bytes!("asset/mount"), 0o777)
    }

    Command::new(mount_prog)
        .args(my_args)
        .spawn()
        .expect("rusty-magisk: ");
}

*/

/*
use sys_mount::{Mount, MountFlags, SupportedFilesystems};
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
            println!("rusty-magisk: Failed to mount device: {}", why);
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
            println!("{}", err_msg);
            exit(exit_code);
        }
    }
}
*/
