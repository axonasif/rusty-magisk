#!/usr/bin/docker
# Repository:       https://github.com/axonasif/rusty-magisk
# Author:           https://githum.com/sickcodes
# License:          GPLv3+

# docker build -t rusty-magisk .
# docker run -it -v "${PWD}/ramdisk.img:/image" rusty-magisk

FROM archlinux:base-devel

MAINTAINER 'https://github.com/sickcodes' <https://sick.codes>

SHELL ["/bin/bash", "-c"]

USER root

ENV USER=root

ENV VERSION=v0.1.7

RUN yes | pacman -Syyu \
    && yes | pacman -S cpio wget --noconfirm

RUN mkdir /ramdisk

WORKDIR /ramdisk

CMD cp /image /image.bak \
    && { zcat /image | cpio -iud \
    && mv /ramdisk/init /ramdisk/init.real \
    && wget -O /ramdisk/init "https://github.com/axonasif/rusty-magisk/releases/download/${VERSION}/rusty-magisk_x86_64" \
    && chmod a+x /ramdisk/init \
    && touch /image \
    && cd /ramdisk \
    && find . /ramdisk | cpio -o -H newc | sudo gzip > /image \
    && echo "Success." ; } \
    || echo "Failed."
