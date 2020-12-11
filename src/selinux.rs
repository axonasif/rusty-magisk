//// 
/// 
/// Ignore this file XD
/// 
/// 

/*
// Now let's deal with selinux if needed
if Path::new("/sys/fs/selinux").exists() {
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
*/
