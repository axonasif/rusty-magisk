# Introduction

This is my very first public `rust` project for fun purposes.

Obviously this was written in  `Rust - a systems programming language` by a noob rustacean with a few hours of experience with rust 😝

My code may seem funny in some parts since I've just tried to apply what I've learnt so far apart from the better ways of doing such tasks

So, kindly ignore as long I'm a noob LoL



# Build instructions

* First of all you need rust. Follow the link below and you should know what to do.

> It'll also autoinstall rustc and cargo for you.

> https://www.rust-lang.org/tools/install

* Once you have rustt and everything it brings, we add a new target. Run the following command:

```bash
rustup target add x86_64-unknown-linux-musl
```

> Or if you wan to make for 32bit arch then the target should be `i686-unknown-linux-musl`.

* After you have the build targets installed simply run the following command:

```bash
cargo build --release --target x86_64-unknown-linux-musl  # For 64bit

# or

cargo build --release --target i686-unknown-linux-musl    # For 32bit
```

> Then you should find the output as `target/x86_64-unknown-linux-musl/release/rusgik`
> I recommend to `strip` the binary for reducing the size.


# Installation

## For Android-9 and below

* Copy your `ramdisk.img` and the `rusgik` binary in an ext4 partition directory and run the following commands:

```bash
su
mkdir ramdisk && ( cd ramdisk && zcat ../ramdisk.img | cpio -iud && mv init init.real )
rsync rusgik ramdisk/init && chmod 777 ramdisk/init && ( cd ramdisk && find . | cpio -o -H newc | gzip > ../ramdisk.img )
```

> Tl;dr: In short, you need to rename `init` executable to `init.real` and put `rusgik` as `init` inside your `ramdisk.img`.

## For Android-10 and above

* If your system image is `system.sfs` then you need to extract it, an quick way to do that is: `7z x system.sfs && rm system.sfs`

* Once you have `system.img` you need to mount it: `mkdir mdir && sudo mount -o loop system.img mdir`

* Now rename `init` to `init.real` by running the following command: `sudo mv mdir/init mdir/init.real`

* Lastly put `rusgik` binary as `init` executable at `/` of system.img: `sudo rsync rusgik mdir/init && chmod 777 mdir/init`


Note: I'm assuming that you have `rusgik` binary at the same dir as your android-x86 OS files.
