# Toolup

Toolup is a CLI that can be used to manage different CLI's.

## Usage
### Init
First you have to `init` toolup. Doing this, you're required to pass in your [GitHub token](https://github.blog/2013-05-16-personal-api-tokens/).

After you've `init`ed the tool, you can start including different CLI's.

### Add Tool
Let's say you want to provision [`ethankhall/crom`](https://github.com/ethankhall/crom). To do this, you'll need to run

_MacOS_
```
toolup manage add-tool --github ethankhall/crom --tgz crom-mac.tar.gz --archive-path crom crom
```

_Linux_
```
toolup manage add-tool --github ethankhall/crom --tgz crom-linux-musl.tar.gz --archive-path crom crom
```

_Windows_
```
toolup.exe manage add-tool --github ethankhall/crom --zip crom-windows.zip --archive-path crom.exe crom.exe
```

Now your tools will download.