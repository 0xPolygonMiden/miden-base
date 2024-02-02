# miden-base

## Testing

To test the different crates we recommend running the following command:

```shell
make test
```

Some of the functions in this project are computationally intensive and may take a significant amount of time to compile and complete during testing. To ensure optimal results, it is recommended to run the tests in release mode, which using specific configurations replicates the test conditions of the development mode and verifies all debug assertions. For more information refer to the [Makefile](./Makefile) for the specific commands that have been chosen.

## License

This project is [MIT licensed](./LICENSE)
