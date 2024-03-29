# SFS (SecureFS)
A shell that allows you to encrypt/decrypt your files with a password

![SFS Showcase](./showcase.png)

SFS encrypts and decrypts files using a fernet (AES-128), generated from a password that you enter every time you open SFS. It also has its own implementation of commands like `ls`, `cp`, `mv`, `rm`, `clear`, and its own command/flag/argument parser.

## Installation
- Requirements:
	- Rust (Cargo)

```sh
git clone https://github.com/ErrorNoInternet/sfs
cd sfs
cargo install --path .
```

SFS has only been tested on Linux, and might not work properly on Windows or other operating systems. Please [create an issue](https://github.com/ErrorNoInternet/sfs/issues/new) if you run into a problem.

<sub>If you would like to modify or use this repository (including its code) in your own project, please be sure to credit!</sub>
