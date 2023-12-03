<h1 align=center>Oatmeal</h1>

![oatmeal](.github/banner.png)

[![Build Status](https://img.shields.io/github/actions/workflow/status/dustinblackman/oatmeal/ci.yml?branch=main)](https://github.com/dustinblackman/oatmeal/actions)
[![Release](https://img.shields.io/github/v/release/dustinblackman/oatmeal)](https://github.com/dustinblackman/oatmeal/releases)
[![Coverage Status](https://coveralls.io/repos/github/dustinblackman/oatmeal/badge.svg?branch=main)](https://coveralls.io/github/dustinblackman/oatmeal?branch=main)

> Terminal UI to chat with large language models (LLM) using different model backends, and integrations with your favourite editors!

- [Overview](#Overview)
- [Install](#Install)
  - [MacOS](#macos)
  - [Debian / Ubuntu](#debian--ubuntu)
  - [Fedora / CentOS](#fedora--centos)
  - [Nix](#nix)
  - [Arch Linux](#arch-linux)
  - [Windows](#windows)
  - [Cargo](#cargo)
  - [Manual](#manual)
  - [Source](#source)
- [Usage](#Usage)
  - [Backends](#backends)
  - [Editors](#editors)
  - [Themes](#themes)
  - [Sessions](#sessions)
- [Development](#Development)
  - [Setup](#setup)
  - [Adding a backend](#adding-a-backend)
  - [Adding an editor](#adding-an-editor)
  - [Adding syntax highlighting for a language](#adding-syntax-highlighting-for-a-language)
- [FAQ](#faq)
  - [Why Oatmeal?](#why-oatmeal)
  - [Report an issue](#report-an-issue)
- [License](#license)

## Overview

Oatmeal is a terminal UI chat application that speaks with LLMs, complete with slash commands and fancy chat bubbles. It features agnostic backends to allow switching between the powerhouse of ChatGPT, or keeping things private with Ollama.
While Oatmeal works great as a stand alone terminal application, it works even better paired with an editor like Neovim!

See it in action with Neovim (click to restart):

![oatmeal-demo](https://github.com/dustinblackman/oatmeal/assets/5246169/9ee5e910-4eff-4deb-8065-aeab8bfe6b00)

_Note:_ This project is still quite new, and LLM's can return unexpected answers the UI isn't prepped for. There's likely a few bugs hidden somewhere.

## Install

### MacOS

```sh
brew install dustinblackman/tap/oatmeal
```

### Debian / Ubuntu

```sh
curl -s https://apt.dustinblackman.com/KEY.gpg | apt-key add -
curl -s https://apt.dustinblackman.com/dustinblackman.list > /etc/apt/sources.list.d/dustinblackman.list
sudo apt-get update
sudo apt-get install oatmeal
```

### Fedora / CentOS

```sh
dnf config-manager --add-repo https://yum.dustinblackman.com/config.repo
dnf install oatmeal
```

### Nix

```sh
nix-env -f '<nixpkgs>' -iA nur.repos.dustinblackman.oatmeal
```

### Arch Linux

```sh
yay -S oatmeal-bin
```

### Windows

**Chocolatey**

Coming soon! Waiting on first version approval.

**Scoop**

```sh
scoop bucket add dustinblackman https://github.com/dustinblackman/scoop-bucket.git
scoop install oatmeal
```

### Cargo

```sh
cargo install oatmeal
```

### Manual

Download the pre-compiled binaries and packages from the [releases page](https://github.com/dustinblackman/oatmeal/releases) and
copy to the desired location.

### Source

```sh
git clone https://github.com/dustinblackman/oatmeal.git
cd oatmeal
cargo build --release
mv ./target/release/oatmeal /usr/local/bin/
```

## Usage

Oatmeal allows customizing configuration either through command line parameters, or more permanently with environment
variables. By default, Ollama is the selected backend, `llama2:latest` as the default model, and the `clipboard` integration for an editor.
See `oatmeal --help`, `/help` in chat, or the output below to get all the details.

<!-- command-help start -->

```
Terminal UI to chat with large language models (LLM) using different model backends, and direct integrations with your favourite editors!

Version: 0.6.0
Commit: v0.6.0

Usage: oatmeal [OPTIONS] [COMMAND]

Commands:
  chat         Start a new chat session
  completions  Generates shell completions
  sessions     Manage past chat sessions
  help         Print this message or the help of the given subcommand(s)

Options:
  -b, --backend <backend>            The initial backend hosting a model to connect to. [Possible values: ollama, openai] [env: OATMEAL_BACKEND=] [default: ollama]
  -m, --model <model>                The initial model on a backend to consume [env: OATMEAL_MODEL=] [default: llama2:latest]
  -e, --editor <editor>              The editor to integrate with. [Possible values: clipboard, neovim] [env: OATMEAL_EDITOR=] [default: clipboard]
  -t, --theme <theme>                Sets code syntax highlighting theme. [Possible values: base16-github, base16-monokai, base16-one-light, base16-onedark, base16-seti] [env: OATMEAL_THEME=] [default: base16-onedark]
      --theme-file <theme-file>      Absolute path to a TextMate tmTheme to use for code syntax highlighting [env: OATMEAL_THEME_FILE=]
      --openai-url <openai-url>      OpenAI API URL when using the OpenAI backend. Can be swapped to a compatiable proxy [env: OATMEAL_OPENAI_URL=] [default: https://api.openai.com]
      --openai-token <openai-token>  OpenAI API token when using the OpenAI backend [env: OATMEAL_OPENAI_TOKEN=]
  -h, --help                         Print help
  -V, --version                      Print version

CHAT COMMANDS:
  - /modellist (/ml) - Lists all available models from the backend.
  - /model (/model) [MODEL_NAME,MODEL_INDEX] - Sets the specified model as the active model. You can pass either the model name, or the index from /modellist
  - /append (/a) [CODE_BLOCK_NUMBER?] - Appends code blocks to an editor. See Code Actions for more details.
  - /replace (/r) [CODE_BLOCK_NUMBER?] - Replaces selections with code blocks in an editor. See Code Actions for more details.
  - /copy (/c) [CODE_BLOCK_NUMBER?] - Copies the entire chat history to your clipboard. When a CODE_BLOCK_NUMBER is used, only the specified copy blocks are copied to clipboard. See Code Actions for more details.
  - /quit /exit (/q) - Exit Oatmeal.
  - /help (/h) - Provides this help menu.

CHAT HOTKEYS:
  - Up arrow - Scroll up
  - Down arrow - Scroll down
  - CTRL+U - Page up
  - CTRL+D - Page down
  - CTRL+C - Interrupt waiting for prompt response if in progress, otherwise exit.
  - CTRL+R - Resubmit your last message to the backend.

CHAT CODE ACTIONS:
When working with models that provide code, and using an editor integration, Oatmeal has the capabilities to read selected code from an editor, and submit model provided code back in to an editor. Each code block provided by a model is indexed with a (NUMBER) at the beginning of the block to make it easily identifiable.

  - /append (/a) [CODE_BLOCK_NUMBER?] will append one-to-many model provided code blocks to the open file in your editor.
  - /replace (/r) [CODE_BLOCK_NUMBER?] - will replace selected code in your editor with one-to-many model provided code blocks.
  - /copy (/c) [CODE_BLOCK_NUMBER?] - will append one-to-many model provided code blocks to your clipboard, no matter the editor integration being used.

The CODE_BLOCK_NUMBER allows you to select several code blocks to send back to your editor at once. The parameter can be set as follows:
  - `1` - Selects the first code block
  - `1,3,5` - Selects code blocks 1, 3, and 5.
  - `2..5`- Selects an inclusive range of code blocks between 2 and 5.
  - None - Selects the last provided code block.
```

<!-- command-help end -->

### Backends

The following model backends are supported:

- [OpenAI](https://chat.openai.com)
- [Ollama](https://github.com/jmorganca/ollama)

### Editors

The following editors are currently supported. The `clipboard` editor is a special case where any copy or accept commands
are simply copied to your clipboard. This is the default behaviour.

- Clipboard
- [Neovim](https://github.com/dustinblackman/oatmeal.nvim)

### Themes

A handful of themes are embedded in the application for code syntax highlighting, defaulting to [OneDark](https://github.com/atom/one-dark-ui). If none suits your needs, Oatmeal supports any Sublime Text/Text Mate
`.tmTheme` file, which can be configured through the `--theme-file` command line parameter, or the `OATMEAL_THEME_FILE`
environment variable. [base16-textmate](https://github.com/chriskempson/base16-textmate) has plenty to pick from!

### Sessions

Oatmeal persists all chat sessions with your models, allowing you to go back and review an old conversation, or pick up
from where you left off!

<!-- command-help-sessions start -->

```
Manage past chat sessions

Usage: oatmeal sessions [OPTIONS] [COMMAND]

Commands:
  dir     Print the sessions cache directory path
  list    List all previous sessions with their ids and models
  open    Open a previous session by ID. Omit passing any session ID to load an interactive selection
  delete  Delete one or all sessions
  help    Print this message or the help of the given subcommand(s)

Options:
  -e, --editor <editor>              The editor to integrate with. [Possible values: clipboard, neovim] [env: OATMEAL_EDITOR=] [default: clipboard]
  -t, --theme <theme>                Sets code syntax highlighting theme. [Possible values: base16-github, base16-monokai, base16-one-light, base16-onedark, base16-seti] [env: OATMEAL_THEME=] [default: base16-onedark]
      --theme-file <theme-file>      Absolute path to a TextMate tmTheme to use for code syntax highlighting [env: OATMEAL_THEME_FILE=]
      --openai-url <openai-url>      OpenAI API URL when using the OpenAI backend. Can be swapped to a compatiable proxy [env: OATMEAL_OPENAI_URL=] [default: https://api.openai.com]
      --openai-token <openai-token>  OpenAI API token when using the OpenAI backend [env: OATMEAL_OPENAI_TOKEN=]
  -h, --help                         Print help
```

<!-- command-help-sessions end -->

Grepping through previous sessions isn't something built in to Oatmeal _(yet)_. This bash function can get you there
nicely using [Ripgrep](https://github.com/BurntSushi/ripgrep) and [FZF](https://github.com/junegunn/fzf).

```bash
function oatmeal-sessions() {
    current=$(pwd)

    cd "$(oatmeal sessions dir)"
    id=$(rg --color always -n . | fzf --ansi | awk -F ':' '{print $1}' | head -n1 | awk -F '.' '{print $1}')
    oatmeal sessions open --id "$id"

    cd $current
}
```

## Development

### Setup

The following will get you set up with all the necessary tooling to work on Oatmeal.

```sh
cargo install cargo-run-bin
git clone https://github.com/dustinblackman/oatmeal.git
cd oatmeal
cargo cmd setup
```

### Adding a backend

Each backend implements the [Backend trait](./src/domain/models/backend.rs) in its own infrastructure file. The trait has documentation on what is expected of each method. You can checkout [Ollama](./src/infrastructure/backends/ollama.rs) as an example.

The following steps should be completed to add a backend:

1. Implement trait for new backend.
2. Update the [BackendManager](./src/infrastructure/backends/mod.rs) to provide your new backend.
3. Write tests
4. Update the documentation for the [CLI](./src/application/cli.rs).

### Adding an editor

Each editor implements the [Editor trait](./src/domain/models/editor.rs) in its own infrastructure file. The trait has documentation on what is expected of each method. You can checkout [Neovim](./src/infrastructure/editors/neovim.rs) as an example.

The following steps should be completed to add an editor:

1. Implement trait for new editor.
2. Update the [EditorManager](./src/infrastructure/editors/mod.rs) to provide your new editor.
3. Write tests
4. Update the documentation for the [CLI](./src/application/cli.rs).

### Adding syntax highlighting for a language

Syntax highlighting language selection is a tad manual where several languages must be curated and then added to
[`build.rs`](./build.rs).

1. Google to find a `.sublime-syntax` project on Github for your language. [bat](https://github.com/sharkdp/bat/tree/master/assets/syntaxes/02_Extra) has many!
2. Update the `get_syntaxes()` function in [`build.rs`](./build.rs) to include the new repo. Make sure to set the URL to
   a commit locked `.tar.gz` file, and to include the license in the files vector.
3. `rm -rf .caches && cargo build`
4. Test to see highlighting works.

## FAQ

### Why Oatmeal?

I was eating a bowl of oatmeal when I wrote the first commit :shrug:. (They don't let me name things at work anymore...)

### Report an issue

On each Oatmeal release there is a separate download to helps in reporting issues to really drill down in to what the
problem is! If you've run in to a problem, I'd really help appreciate solving it.

1. Head over to [releases](https://github.com/dustinblackman/oatmeal/releases) and download the DEBUG package for your
   system and version of Oatmeal you're using.
2. Extract the contents of the archive next to your oatmeal binary. You can find it on your system by running something
   equivalent to `which oatmeal`.
3. Run your command with the arguments provided in the error message.
4. Copy/paste the output and [open an issue](https://github.com/dustinblackman/oatmeal/issues/new).

## License

[MIT](./LICENSE)
