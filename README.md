# Baumstamm

Baumstamm is an application to create, view and edit complex family trees.
Beware, it is in early development.
The base functionality is available,
but more advanced features are still in the making.

## Why use Baumstamm?

With the most genealogy software you quickly run into limitations.
Often, you can just model your direct ancestors or descendants
and relationships between different ends of the tree are impossible to add.
In contrast to that, you can create arbitrarily complex family trees with Baumstamm.

Baumstamm is free and open source.
You can run it locally on your computer
or use the static [website](https://alecghost.github.io/baumstamm/);
no data leaves your device.
The family tree is stored as plain text JSON,
so you can easily use it with other software.

## Manual Installation

Make sure you have [Rust](https://rustup.rs/)
and [npm](https://docs.npmjs.com/downloading-and-installing-node-js-and-npm) installed.
Furthermore, you need the Tauri CLI.

```sh
cargo install tauri-cli
```

Start the manual installation by cloning this repository.

```sh
git clone https://github.com/AlecGhost/baumstamm.git
```

In the app directory, install all required node dependencies.

```sh
cd baumstamm/baumstamm-app
npm install
```

Then go back to the root directory of this repository and build the app.

```sh
cd ..
cargo tauri build
```

Now you're ready to go!

## Licensing

The core libraries are available under the Apache 2.0 license,
while the GUI and CLI applications are under GPL 3.0.
