# Configuration

Before TSFFS can fuzz target software, it must be configured. The configuration API is
kept as simple as possible, with sane defaults. TSFFS exposes all of its configuration
options as Simics *attributes* which means that you can list its configuration options
by running the following in a Simics CLI prompt in a project with TSFFS installed (see
[Installing in Projects](installing-in-projects.md)).

```simics
load-module tsffs
list-attributes tsffs
```

You'll see a list of attributes, each of which has help documentation available through
the Simics CLI like:

```simics
help tsffs.exceptions
```

To read about all of the TSFFS options in detail, including methods for setup,
installation, and configuration:

- [Installing In Projects](installing-in-projects.md)
- [Loading The TSFFS Module](loading-module.md)
- [Common Options](common-options.md)