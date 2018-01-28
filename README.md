## Setup
### Host setup
#### Mac
```bash
brew install nanomsg --HEAD
brew install python3 rsync
```

### VM setup
[VirtualBox](https://www.virtualbox.org/) is the preferred virtualizer, although any other virtualizer that can provided shared folders and port forwarding will also work. 

[Debian 9 Stretch](https://wiki.debian.org/DebianStretch) is the distro of choice. 

* Set up port forwarding  

	22 → 2222 (SSH forward) 

	**SSH keys are recommended**

* Set up shared folders  
	
	→ `Folder name`: 9001d  
	→ `Path`: <absolute path to 9001d>  	
* Enable symlinks in shared folders  

	```bash
	VBoxManage setextradata <vm-name> VBoxInternal2/SharedFoldersEnableSymlinksCreate/<share-name> 1
	```  

	Restart VirtualBox to apply this change.

* Start VM

* Insert guest additions CD

* Install guest additions

	```bash
	# Dependencies
	sudo apt-get install curl build-essential module-assistant dkms pkg-config

	# Prepare system to build kernel modules
	sudo m-a prepare

	# Mount Guest additions CD and build
	sudo mount /dev/sr0 /mnt
	cd /mnt
	sudo ./VBoxLinuxAdditions.run

	# Allow current user to access shared folders
	sudo adduser "$USER" vboxsf

	# Reboot
	sudo reboot
	```

* (*Optional*) Setup environment variables

	```bash
	make env
	```

	Use something like [direnv](https://github.com/direnv/direnv) to automatically run this file.

* Setup cross-compile environment

	```bash
	make setup
	```


## Cross-compiling
Use `make build` to generate foreign binaries and `make transfer` to copy them over to the target.

### Environment Variables
* `TARGET_ADDRESS`  
	Address of machine that the compiled binary should be deployed on.  
	Example: `pigeon9001.local`
* `TARGET_BIN_LOCATION`  
	Target location of the deployed binary.  
	Example: `'~'`
* `TARGET_USER`  
	Username on target system.  
	Example: `philip`
* `VM_PORT`  
	SSH port of cross-compile VM.  
	Example: `2222`
* `VM_PROJECT_LOCATION`  
	Shared project folder location.  
	Example: `/media/sf_9001d`
* `VM_USER`  
	Username on cross-compile VM.  
	Example: `philip`
* `CONFIGURATION`  
	Defaults to `debug`, set to `release` for optimized builds.  
	Example: `release`
* `TARGET`  
	Defaults to `armv7`, set to `arm` for ARMv6 builds.  
	Example: `arm`