# miden-base

## Testing

To test the crates contained in this repo, you can run the following command:

```shell
make test
```

Some of the functions in this project are computationally intensive and may take a significant amount of time to compile and complete during testing. To ensure optimal results we use the `make test` command. It enables the running of tests in release mode and using specific configurations replicates the test conditions of the development mode and verifies all debug assertions. For more information refer to the [Makefile](./Makefile) for the specific commands and configurations that have been chosen.

## License

This project is [MIT licensed](./LICENSE)
