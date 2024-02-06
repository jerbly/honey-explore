# 0.1.7

- Fix: Timerange in queries was sometimes rejected for being too long.

# 0.1.6

- Number type attributes now run an `avg` query when a dataset is clicked
- Template type attributes:
    - Now render with the `.<key>` suffix in the name
    - The dictionary, discovered from Honeycomb, is displayed in a `keys` section with query links
- Added a favicon


# 0.1.5

- Uses `cargo-dist` for build and release.

# 0.1.4

- Moved the Honeycomb API calls to a separate crate.
