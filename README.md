# yuque-squirrel

Yuque backup utilities.

## Configuration

Copy `config-template.json` to anywhere and modify its content.

Note that the `host` field only accepts URLs that is not suffixed with `/`, or the URL parsing will fail. This may be fixed in the future.

## Usage

Use `yuque-squirrel -c <CONFIG_PATH> <PATH>` to start the backup process.

The backup process is incremental, which means that it will only download new or updated documents.

This program is single-threaded, but it's async, so it should be fast enough, although with blocking filesystem operations.
