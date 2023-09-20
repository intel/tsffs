# FAQ

Questions we receive from Discord, via email, and as Issues or Discussions in the
repository will be periodically added here.

- [FAQ](#faq)
  - [Q: Compile error: `dotenv!` is complaining](#q-compile-error-dotenv-is-complaining)
  - [Q: Can I stop and resume the fuzzer?](#q-can-i-stop-and-resume-the-fuzzer)
  - [Q: "An error was encountered while trying to build the list of packages to install"](#q-an-error-was-encountered-while-trying-to-build-the-list-of-packages-to-install)
  - [Q: Can I contribute to ths project?](#q-can-i-contribute-to-ths-project)

## Q: Compile error: `dotenv!` is complaining

<!-- Internally asked -->

If you see this error:

```text
macro expansion ignores token `;` and any following
the usage of `dotenv!` is likely invalid in expression context
```

Under a line of code like:

```rust
const SIMICS_HOME: &str = dotenv!("SIMICS_HOME");
```

All you need to do is follow the [setup guide](./Setup.md#set-up-simics_home) and set
up your `.env` file.

## Q: Can I stop and resume the fuzzer?

<!-- Asked on Discord -->

Yes, passing `--input CORPUS_DIRECTORY --corpus CORPUS_DIRECTORY`, where
`CORPUS_DIRECTORY` is a directory containing your input seed files will make the fuzzer
save the corpus, which consists of inputs the fuzzer has found which trigger new
coverage in the target software, back to the input directory. This means when you run
the fuzzer again with the same set of arguments, it will load the corpus it was working
on before, effectively "resuming" from where it left off.

## Q: "An error was encountered while trying to build the list of packages to install"

<!-- Asked on Discord -->

If you see "An error was encountered while trying to build the list of packages to
install" when running `ispm` to install SIMICS packages, you should try re-downloading
the SIMICS packages ISPM, it's likely the `.ispm` file is corrupted or incompletely
downloaded.

## Q: Can I contribute to ths project?

<!-- Asked on Discord -->

Absolutely! Please contribute, and if you have an idea that you want help implementing
or do not have time to implement, please create an issue with the "enhancement" tag so
the maintainer team can track it.


