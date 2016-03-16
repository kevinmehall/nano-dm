# nano-dm

Receive Qualcomm DSP debug logs over USB

`nano-dm` is written in [Rust](https://www.rust-lang.org).

Install Rust:
```
curl -sSf https://static.rust-lang.org/rustup.sh | sh
```

Install Libusb (OSX):
```
brew install libusb
```

Install Libusb (Ubuntu):
```
sudo apt-get install libusb-1.0-0-dev
```

Clone & build:
```
git clone https://github.com/kevinmehall/nano-dm.git
cd nano-dm
cargo build --release
```

Run:
```
./target/release/nano-dm
```
