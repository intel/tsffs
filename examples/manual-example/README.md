# Manual Example

This example shows how to manually harness a binary in black-box mode
(that is, without being able to compile in a harness).

To run this example, from the *repo root* run:

```sh
docker build -t tsffs-manual -f examples/manual-example/Dockerfile .
docker run -it tsffs-manual
```

Then in the container run:

```sh
./simics -no-gui --no-win --batch-mode fuzz.simics
```