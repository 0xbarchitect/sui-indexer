# LibPQ
How to install libpq for Postgres connection on MacOSX

## Install

```sh
$ brew install libpq
```

## Build flags

- Add these build flags to `.zshrc` file

```bash
export LDFLAGS="-L/Users/user/homebrew/opt/libpq/lib"
export CPPFLAGS="-I/Users/user/homebrew/opt/libpq/include"
export LIBRARY_PATH="$LIBRARY_PATH:/Users/user/homebrew/opt/libpq/lib"
```

*note: revise the path to libpq according to your computer*