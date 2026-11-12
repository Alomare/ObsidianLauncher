#!/bin/sh
# Builds the LuaJIT 2.0 compiler Obsidian uses into target/luajit (needs git, make and a C compiler).
set -e
root=$(cd "$(dirname "$0")/.." && pwd)
src=$root/target/luajit-src
out=$root/target/luajit
# A cache can bring the folder back without a working .git; git would then act on the Obsidian repo.
if [ ! -f "$src/.git/HEAD" ]; then
  rm -rf "$src"
  git clone --quiet https://github.com/LuaJIT/LuaJIT "$src"
fi
git -C "$src" checkout --quiet 2410b57
make -C "$src" -j"$(nproc)" BUILDMODE=static
mkdir -p "$out/lua/jit"
cp "$src/src/luajit" "$out/"
cp "$src"/src/jit/*.lua "$out/lua/jit/"
echo "LuaJIT 2.0 is ready in $out"
