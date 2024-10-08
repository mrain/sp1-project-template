# List of commands
default:
    just --list

# Build SP1 program under sp1/program
@sp1-build:
    echo "Rebuilding SP1 program ..."
    cd program && cargo-prove prove build
    echo "... done"

# Generate and verify SP1 proof
@sp1-prove *args: sp1-build
    echo "Proving & Verifying SP1 program ..."
    cd script && RUST_LOG=info cargo run --bin prove --release -- {{args}}
    echo "... done"

# Bench the SP1 prover
@sp1-bench *args: sp1-build
    echo "Proving & Verifying SP1 program ..."
    cd script && RUST_LOG=info cargo run --bin prove --release -- --bench {{args}}
    echo "... done"
