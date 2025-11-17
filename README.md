# zkp-merlin-sgx
To run Intel SGX and Fortanix EDP in Azure, you should use DC-series virtual machines,
which provide SGX support and expose the required kernel devices under /dev/sgx*.

The test was performed on the Standard_DC1s_v3 instance.
# repo SGX
```sh
echo "deb https://download.01.org/intel-sgx/sgx_repo/ubuntu $(lsb_release -cs) main" \
| sudo tee -a /etc/apt/sources.list.d/intel-sgx.list >/dev/null

curl -sSL "https://download.01.org/intel-sgx/sgx_repo/ubuntu/intel-sgx-deb.key" \
| sudo -E apt-key add -

sudo apt-get update
```
# AESM
```sh
sudo apt-get install -y sgx-aesm-service libsgx-aesm-launch-plugin
```
# dev tools
```sh
sudo apt-get install -y build-essential pkg-config libssl-dev protobuf-compiler
```
# rustup + nightly + SGX target
```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup default nightly
rustup target add x86_64-fortanix-unknown-sgx --toolchain nightly
```

# fortanix tools
```sh
cargo install fortanix-sgx-tools sgxs-tools
```
# global cargo runner
```sh
mkdir -p ~/.cargo
cat <<EOF > ~/.cargo/config.toml
[target.x86_64-fortanix-unknown-sgx]
runner = "ftxsgx-runner-cargo"
EOF
```


# Run 

To run project please perform 

```sh
cargo run 
```
## Run compiled file
to create release:
```sh
cargo build --release --target x86_64-fortanix-unknown-sgx
```

then
```sh
ftxsgx-runner target/x86_64-fortanix-unknown-sgx/release/marlin-sgx-test.sgxs
```

## Run compiled file in debug mode

```sh
cargo build --target x86_64-fortanix-unknown-sgx
```

then
```
ftxsgx-runner target/x86_64-fortanix-unknown-sgx/debug/marlin-sgx-test.sgxs
```