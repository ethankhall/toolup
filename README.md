# Toolup

Toolup is a CLI that can be used to manage different CLI's.

## Install

Currently the only install is via cargo.

`cargo install toolup --git https://github.com/ethankhall/toolup`

In order to get the tools on you're path, you'll need need to add the
following to your profile.

```shell
export PATH=`$HOME/.toolup/bin/`:$PATH
```

This will ensure that the binaries that get added will be on your path.

## Config

In general, an IT department will be managing config. The config should be placed based
on the following table.

|    OS    |        Path         |
| :------: | :-----------------: |
| `darwin` | `~/.config/toolup/` |
| `linux`  | `~/.config/toolup/` |

If an IT department would loke to use a different location, they will need to set
`TOOLUP_GLOBAL_CONFIG_DIR`.

The config should be generated by using `toolup remote add [local|s3] {args}` to ensure
that the file is correct. Once it's been created, it is safe to move to other machines.

The S3 backed artifacts may have a script to authenticate with S3, if so that file will also need to be located on other machines.

## Packages

Creating a package is easy! Use `toolup package init` and a `package.toml` will be created.
Inside the `package.toml` update the name and list all the binaries the package provides.

The binaries are "relative" to the tool directory when packaging. For easiest management,
you should put the package.toml at the root of the folder structure you'll put the binaries
in.

When in doubt, use the [Unix Filesystem Hierarchy](https://en.wikipedia.org/wiki/Filesystem_Hierarchy_Standard) to structure the package.

When the package is ready, run `toolup package archive` and point it at the config file,
package directory, and output directory. This will create a `{name}.tar.gz` file. This can
be used to install the package.