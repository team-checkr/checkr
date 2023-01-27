# Checko

Infrastructure for evaluating external implementations.

## Introduction

Checko is designed around three core configuration files: `programs.toml`, `groups.toml`, and `run.toml`. These define the input parameters for evaluating groups, and how to compile and run each external implementation.

Using Checko is done in three stages:

### Running tests

The first stage is running tests for all defined projects are downloaded, put into a Docker container, where they are compiled, and all sample programs are run to produce an output. This output is then saved to disk for usage in the two later stages.

This stage can be run as follows:

```bash
checko run-tests -p programs.toml -g groups.toml -s submissions/
```

### Giving feedback

Based on the run tests, a Markdown file is generated and pushed to a separate branch in each project.

This stage can be run as follows:

```bash
checko push-results-to-repos -g groups.toml -s submissions/
```

By default, this _will not push_ to the remote repo, but rather is a dry-run of the execution. To perform the actual publishing, append a `--execute` to the previous command.

### Generating competition results

Similar to the previous stage, this stage uses prior runs to generate a summary of all the runs in a single file, to compare progress across projects.

This stage can be run as follows:

```bash
checko generate-competition -g groups.toml -s submissions/ -o competition-results.md
```

Here, the output file of the competition must be specified as well.

## Example setup

To see an example of how this can all be setup, checkout the [`example`](./example/) folder! It contains a `Justfile` for running the four basic interactions that are required.
