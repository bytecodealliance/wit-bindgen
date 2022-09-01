printf "Running 'postCreateCommand' Script\n"

# Install Rust Targets
printf "Installing Rust Targets\n"
rustup update stable --no-self-update
rustup default stable
rustup target add wasm32-unknown-unknown
rustup target add wasm32-wasi

# Install Python stuff
printf "Installing Python Dependencies"
pip install mypy wasmtime

# Install NPM dependencies
printf "Installing NPM Dependencies"
cd crates/gen-host-js && npm install
