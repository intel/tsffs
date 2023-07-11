# Clang-Format

if ! command -v cargo &>/dev/null; then
    echo "cargo must be installed! Install from https://rustup.rs"
    exit 1
fi

if ! command -v fd &>/dev/null; then
    echo "fd must be installed! Install with 'cargo install fd-find'"
    exit 1
fi

if ! command -v clang-format &>/dev/null; then
    echo "clang-format must be installed! Install with your system package manager."
    exit 1
fi

echo "Formatting C/C++"

fd '.*(\.h|\.c|\.cc|\.hh)$' -x clang-format -i {}

echo "Formatting Rust"

cargo fmt --all
