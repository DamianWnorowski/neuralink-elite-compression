#!/bin/bash
# Neuralink Compression Challenge - Build Script
# This script builds the Ouroboros Elite Compressor and creates encode/decode wrappers.

# Ensure we are in the correct directory
cd source || exit 1

# Build the Rust project in release mode
cargo build --release

# Move back to root
cd ..

# Create wrappers for encode and decode
cat <<EOF > encode
#!/bin/bash
./source/target/release/neuralink_compressor encode "\$@"
EOF

cat <<EOF > decode
#!/bin/bash
./source/target/release/neuralink_compressor decode "\$@"
EOF

# Make them executable
chmod +x encode decode

echo "Build complete. Use ./encode and ./decode to run the compressor."
