# Developing

This is a short set of formatting tips for this project to keep formatting consistent.

## Formatting

There is a formatting configuration in [.clang-format](../.clang-format) that provides
a C/C++ code format and can be automatically or manually applied several ways.

### Installing ClangD

Some editor plugins will install clangd for you. If they don't, you can obtain clangd for
your distribution [from LLVM](https://apt.llvm.org/).

### Editor Plugins

Many editors provide plugins to automatically format your code as you write using this
format:

- [VSCode](https://marketplace.visualstudio.com/items?itemName=llvm-vs-code-extensions.vscode-clangd)
- [Sublime Text](https://github.com/sublimelsp/LSP-clangd)
- [VIM](https://github.com/ycm-core/YouCompleteMe)

Other editors and methods are mentioned [in the LLVM docs for clangd](https://releases.llvm.org/9.0.1/tools/clang/tools/extra/docs/clangd/Installation.html)

### Manually Formatting

If you prefer to manually format your code, you can do so by using `clang-format` from
the command line:

```bash
$ clang-format -i /path/to/c_or_cpp_file
```

## Pre-Commit

Pre-Commit hooks run checks to make sure your code is formatted correctly before
committing so you don't have to worry about accidentally committing un-formatted code.

You can install pre-commit and the `clang-format` hook with:

```bash
$ python3 -m pip install pre-commit
$ pre-commit install
```

Now, whenever you run `git commit`, the format check will run. If your formatter is
running automatically via an editor plugin, this check is unlikely to ever fail. If you
are running `clang-format` manually, it may ask you to run it and commit the
now-formatted files.

## Editor Settings

Basic vscode settings are provided to configure C/C++ plugins automatically.