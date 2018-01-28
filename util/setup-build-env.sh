#!/bin/bash

set -e
set -x

sudo apt-get update

# Base Dependencies
sudo apt-get install curl build-essential pkg-config subversion

# rustup
if ! [ -x "$(command -v rustup)" ]; then
	curl https://sh.rustup.rs -sSf > rustup.sh
	chmod +x rustup.sh
	./rustup.sh -y
	rm rustup.sh
fi


# ARMv7 toolchain
sudo dpkg --add-architecture armhf
sudo apt-get update
sudo apt-get install crossbuild-essential-armhf


# ARMv6 toolchain
if [ ! -d .cargo/armv6-toolchain ]; then
	svn export https://github.com/raspberrypi/tools/trunk/arm-bcm2708/gcc-linaro-arm-linux-gnueabihf-raspbian/ .cargo/armv6-toolchain
fi

rustup target add arm-unknown-linux-gnueabihf
rustup target add armv7-unknown-linux-gnueabihf

# nanomsg cross-compile
mkdir -p deps
cd deps

mkdir -p nanomsg
cd nanomsg

if [ ! -d nanomsg-src ]; then
	git clone --depth=1 https://github.com/nanomsg/nanomsg.git nanomsg-src
else 
	git -C nanomsg-src fetch --all
	git -C nanomsg-src reset --hard origin/master

fi

cd nanomsg-src

mkdir -p build
cd build

# Delete previous build
rm -rf build-armv6

mkdir -p build-armv6
cd build-armv6

cmake -DNN_STATIC_LIB=ON -DNN_ENABLE_GETADDRINFO_A=OFF -DNN_TOOLS=OFF -DNN_ENABLE_DOC=OFF -DCMAKE_TOOLCHAIN_FILE=../../../util/armv6-toolchain.cmake ../..
make DESTDIR=../../../arm install

cd ..

# Delete previous build
rm -rf build-armv7

mkdir -p build-armv7
cd build-armv7

cmake -DNN_STATIC_LIB=ON -DNN_ENABLE_GETADDRINFO_A=OFF -DNN_TOOLS=OFF -DNN_ENABLE_DOC=OFF -DCMAKE_TOOLCHAIN_FILE=../../../util/armv7-toolchain.cmake ../..
make DESTDIR=../../../armv7 install
