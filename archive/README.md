# archive

# Building

Depends on `libarchive`: http://www.libarchive.org/

## macOS

Install `libarchive` and `pkgconfig`:
```shell
brew install libarchive@3.6.1 pkgconfig
```

Configure pkgconfig to locate `libarchive`:
```shell
# Note: The default homebrew location is already configured in .cargo/config.toml
export PKG_CONFIG_PATH=$(brew ls libarchive | grep .pc$ | sed 's|/libarchive.pc||')
```

Run `cargo build`.

## Linux



## Windows
