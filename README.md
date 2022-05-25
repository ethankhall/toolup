# Toolup

`toolup` is a CLI that can be used to manage different CLI's.
`toolup` is not intended to replace a normal package manager, instead allow organizations to install versioned applications.
The major usecase for `toolup` is to deploy versions to a machine, and when there is a bug that effects only a small number of users, that user will be able to roll back to an older version locally without requireing other tooling to change.

## TL;DR

If you're a user, run `toolup remote update` to update your tools.
If that doesn't work, reach out to the team that manages `toolup`

## Install

Currently the only install is via cargo.

`cargo install toolup --git https://github.com/ethankhall/toolup`

In order to get the tools on you're path, you'll need need to add the following to your profile.

```shell
export PATH="$HOME/.toolup/bin/:$PATH"
```

This will ensure that the binaries that get added will be on your path.

## User Operations

Assuming that someone else (and IT department) manages the config files, the user will only need to run `toolup remote update`. This will update their local applications based on the provided config.


## Management

### Config

In general, an IT department will be managing config. The config should be placed based
on the following table.

|    OS    |        Path         |
| :------: | :-----------------: |
| `darwin` | `~/.toolup/config/` |
| `linux`  | `~/.toolup/config/` |

If an IT department would loke to use a different location, they will need to set
`TOOLUP_GLOBAL_CONFIG_DIR`.

The config should be generated by using `toolup remote add [local|s3] {args}` to ensure that the file is correct. Once it's been created, it is safe to move to other machines.

The S3 backed artifacts may have a script to authenticate with S3, if so that file will also need to be located on other machines.

### Packages

Creating a package is easy! Use `toolup package init` and a `package.toml` will be created.
Inside the `package.toml` update the name and list all the binaries the package provides.

The binaries are "relative" to the tool directory when packaging. For easiest management, you should put the package.toml at the root of the folder structure you'll put the binaries in.

When in doubt, use the [Unix Filesystem Hierarchy](https://en.wikipedia.org/wiki/Filesystem_Hierarchy_Standard) to structure the package.

When the package is ready, run `toolup package archive` and point it at the config file, package directory, and output directory. This will create a `{name}.tar.gz` file. This can be used to install the package.

Standard usage would look like the following snippit.

```bash
toolup package archive \
    --config <path>/config/package.toml \
    --target-dir <path>/config \
    --archive-dir <path>
```

To see an example of how to create the archive review [test/install-tools.sh](./test/install-tools.sh).

### Debugging

By default, the output to the user is fairly limited.
This is to make the tool easy to unserstand.

Logs will also be written to `~/.toolup/logs`.
The output is JSON and there is much more info about what's going on.

If the user would like to see the output that would go to logs, they can add `--console/-c` to their command and all output will endup on STDOUT.

Adding `-d` will increase the logging. This can be used multiple times.

In order to view the logs as readable text, we recomment [`pino-pretty`](https://github.com/pinojs/pino-pretty).
Using `pino-pretty` like `cat ~/.toolup/logs/toolup.log.2022-05-24-23 | pino-pretty -m message`.