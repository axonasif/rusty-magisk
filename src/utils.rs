use nix::unistd;
use std::ffi::CString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use sys_mount::{Mount, MountFlags};

pub fn executev(args: &[&str]) {
    let args: Vec<CString> = args
        .iter()
        .map(|t| CString::new(*t).expect("rusty-magisk: Not a proper CString"))
        .collect();
    unistd::execv(&args[0], &args).expect("rusty-magisk: Failed to complete executev() call");
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
    let init_real = "/init.real";
    if Path::new(init_real).exists() {
        executev(&[init_real]);
    }
}

pub fn remount_root() {
    match Mount::new("/", "/", "", MountFlags::REMOUNT, None) {
        Ok(_) => {}
        Err(_) => {}
    }
}

////// Some unused functions below

/*
pub fn clone_perms(source: &str, target: &str) -> std::io::Result<()> {
    let perms = fs::metadata(source)?.permissions();
    fs::set_permissions(target, perms)?;
    Ok(())
}

// OverlayFS really cant be mounted once first initrd does either chroot/switch_root into `/android`
// I've wasted many hours over this and had to finally ditch the idea :(
pub fn create_overlay() {
    // Transform /system/bin into overlayFS mountpoint
    for dir in ["/dev/upper", "/dev/work"].iter() {
        fs::create_dir_all(dir).expect("rusty-magisk: Failed to setup bin_dir at /dev");
    }

    clone_perms("/system/bin", "/dev/upper").expect("Failed to clone /android perms");

    libmount::Overlay::writable(
        ["/system/bin"].iter().map(|x| x.as_ref()),
        "/dev/upper",
        "/dev/work",
        "/system/bin",
    )
    .mount()
    .expect("rusty-magisk: Failed to setup overlayFS at /system/bin");
}
*/

/*
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
